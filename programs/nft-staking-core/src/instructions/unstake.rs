use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};
use mpl_core::{
    instructions::{RemovePluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attributes, FreezeDelegate, Plugin, PluginType},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    attributes::{accrued_rewards, assert_asset, asset_attributes, set_attribute, staked_at},
    error::ErrorCode,
    Config, CONFIG_SEED, REWARDS_SEED, STAKED_AT_KEY, UPDATE_AUTHORITY_SEED,
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Core-owned; ownership and collection checked in `unstake`.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core-owned; bound to config and update authority via seeds.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer; holds the FreezeDelegate and Attributes authority.
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

impl Unstake<'_> {
    pub fn unstake(&self, bumps: &UnstakeBumps) -> Result<()> {
        let asset = self.asset.to_account_info();
        assert_asset(&asset, &self.collection.key(), &self.owner.key())?;

        let mut attribute_list = asset_attributes(&asset).ok_or(ErrorCode::NotStaked)?;
        let staked_at = staked_at(&attribute_list).ok_or(ErrorCode::NotStaked)?;
        let elapsed = Clock::get()?.unix_timestamp - staked_at;
        require!(
            elapsed >= self.config.min_stake_duration,
            ErrorCode::StakeLocked
        );
        let amount = accrued_rewards(self.config.rewards_per_day, elapsed)?;

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            UPDATE_AUTHORITY_SEED,
            collection_key.as_ref(),
            &[bumps.update_authority],
        ]];

        // A frozen FreezeDelegate rejects its own removal, so thaw first.
        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;

        // Removing an owner-managed plugin needs the owner; the delegate can only use it.
        RemovePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin_type(PluginType::FreezeDelegate)
            .invoke()?;

        set_attribute(&mut attribute_list, STAKED_AT_KEY, "0".to_string());
        UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.update_authority.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::Attributes(Attributes { attribute_list }))
            .invoke_signed(signer_seeds)?;

        let config_seeds: &[&[&[u8]]] =
            &[&[CONFIG_SEED, collection_key.as_ref(), &[self.config.bump]]];
        mint_to(
            CpiContext::new_with_signer(
                self.token_program.key(),
                MintTo {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.owner_rewards_ata.to_account_info(),
                    authority: self.config.to_account_info(),
                },
                config_seeds,
            ),
            amount,
        )
    }
}
