use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use mpl_core::{
    instructions::{BurnV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    attributes::{assert_asset, asset_attributes, rewards_since},
    error::ErrorCode,
    rewards::{accrued_rewards, mint_rewards},
    Config, BURN_BONUS_DAYS, CONFIG_SEED, REWARDS_SEED, UPDATE_AUTHORITY_SEED,
};

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Core-owned; ownership and collection checked in `burn_staked_nft`.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core-owned; bound to config and update authority via seeds.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer; holds the FreezeDelegate and BurnDelegate authority.
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump = config.rewards_bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = rewards_mint,
        associated_token::authority = owner,
        associated_token::token_program = token_program
    )]
    pub owner_rewards_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl BurnStakedNft<'_> {
    pub fn burn_staked_nft(&self, bumps: &BurnStakedNftBumps) -> Result<()> {
        let asset = self.asset.to_account_info();
        assert_asset(&asset, &self.collection.key(), &self.owner.key())?;

        let attribute_list = asset_attributes(&asset).ok_or(ErrorCode::NotStaked)?;
        let since = rewards_since(&attribute_list).ok_or(ErrorCode::NotStaked)?;
        let now = Clock::get()?.unix_timestamp;
        let bonus = self
            .config
            .rewards_per_day
            .checked_mul(BURN_BONUS_DAYS)
            .ok_or(ErrorCode::Overflow)?;
        let amount = accrued_rewards(self.config.rewards_per_day, now - since)?
            .checked_add(bonus)
            .ok_or(ErrorCode::Overflow)?;

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            UPDATE_AUTHORITY_SEED,
            collection_key.as_ref(),
            &[bumps.update_authority],
        ]];

        // Core resolves lifecycle checks as "any Reject wins", so the frozen FreezeDelegate
        // would veto the BurnDelegate's Approve. Thaw first, then burn in the same tx.
        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;

        // The PDA burns through its BurnDelegate authority; the owner only pays and consents.
        BurnV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        mint_rewards(
            &self.token_program,
            &self.rewards_mint,
            &self.owner_rewards_ata,
            &self.config,
            amount,
        )
    }
}
