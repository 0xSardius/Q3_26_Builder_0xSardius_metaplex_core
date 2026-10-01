use anchor_lang::prelude::*;
use mpl_core::{
    instructions::{AddPluginV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{Attributes, FreezeDelegate, Plugin, PluginAuthority},
    ID as CORE_PROGRAM_ID,
};

use crate::{
    attributes::{assert_asset, asset_attributes, set_attribute, staked_at},
    error::ErrorCode,
    Config, CONFIG_SEED, LAST_CLAIMED_AT_KEY, STAKED_AT_KEY, UPDATE_AUTHORITY_SEED,
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Core-owned; ownership and collection checked in `stake`.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core-owned; bound to config and update authority via seeds.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer for authority-managed plugins.
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump)]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl Stake<'_> {
    pub fn stake(&self, bumps: &StakeBumps) -> Result<()> {
        let asset = self.asset.to_account_info();
        assert_asset(&asset, &self.collection.key(), &self.owner.key())?;

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            UPDATE_AUTHORITY_SEED,
            collection_key.as_ref(),
            &[bumps.update_authority],
        ]];
        let now = Clock::get()?.unix_timestamp;

        // Attributes is authority-managed: only the update authority (our PDA) can write it.
        match asset_attributes(&asset) {
            None => {
                let mut attribute_list = Vec::new();
                set_attribute(&mut attribute_list, STAKED_AT_KEY, now.to_string());
                set_attribute(&mut attribute_list, LAST_CLAIMED_AT_KEY, now.to_string());
                AddPluginV1CpiBuilder::new(&self.core_program.to_account_info())
                    .asset(&asset)
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.owner.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes { attribute_list }))
                    .invoke_signed(signer_seeds)?;
            }
            Some(mut attribute_list) => {
                require!(staked_at(&attribute_list).is_none(), ErrorCode::AlreadyStaked);
                set_attribute(&mut attribute_list, STAKED_AT_KEY, now.to_string());
                set_attribute(&mut attribute_list, LAST_CLAIMED_AT_KEY, now.to_string());
                UpdatePluginV1CpiBuilder::new(&self.core_program.to_account_info())
                    .asset(&asset)
                    .collection(Some(&self.collection.to_account_info()))
                    .payer(&self.owner.to_account_info())
                    .authority(Some(&self.update_authority.to_account_info()))
                    .system_program(&self.system_program.to_account_info())
                    .plugin(Plugin::Attributes(Attributes { attribute_list }))
                    .invoke_signed(signer_seeds)?;
            }
        }

        // FreezeDelegate is owner-managed: the owner must sign to add it, then hands
        // its authority to our PDA so only the program can thaw.
        AddPluginV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&asset)
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .system_program(&self.system_program.to_account_info())
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::Address {
                address: self.update_authority.key(),
            })
            .invoke()?;

        Ok(())
    }
}
