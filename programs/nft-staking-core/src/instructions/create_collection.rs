use anchor_lang::prelude::*;
use mpl_core::{
    instructions::CreateCollectionV2CpiBuilder,
    types::{
        Attribute, Attributes, ExternalPluginAdapterInitInfo, HookableLifecycleEvent,
        OracleInitInfo, Plugin, PluginAuthorityPair, ValidationResultsOffset,
    },
    ExternalCheckResultBits, ID as CORE_PROGRAM_ID,
};

use crate::{ORACLE_SEED, TOTAL_STAKED_KEY, UPDATE_AUTHORITY_SEED};

#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Fresh keypair; Core allocates and initializes it.
    #[account(mut)]
    pub collection: Signer<'info>,

    /// CHECK: data-less PDA that becomes the collection's update authority.
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    /// CHECK: only its address is recorded; it may not be initialized yet.
    #[account(seeds = [ORACLE_SEED], bump)]
    pub oracle: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl CreateCollection<'_> {
    pub fn create_collection(&self, name: String, uri: String) -> Result<()> {
        // `authority: None` defaults Attributes to the collection's update authority (our PDA).
        let stats = PluginAuthorityPair {
            plugin: Plugin::Attributes(Attributes {
                attribute_list: vec![Attribute {
                    key: TOTAL_STAKED_KEY.to_string(),
                    value: "0".to_string(),
                }],
            }),
            authority: None,
        };

        // Core consults the oracle on every Transfer of every asset in the collection.
        // Reject-only: the oracle can veto a transfer but never approve one on its own.
        let transfer_oracle = ExternalPluginAdapterInitInfo::Oracle(OracleInitInfo {
            base_address: self.oracle.key(),
            init_plugin_authority: None,
            lifecycle_checks: vec![(
                HookableLifecycleEvent::Transfer,
                ExternalCheckResultBits::new().with_can_reject(true).into(),
            )],
            base_address_config: None,
            results_offset: Some(ValidationResultsOffset::Anchor),
        });

        CreateCollectionV2CpiBuilder::new(&self.core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .update_authority(Some(&self.update_authority.to_account_info()))
            .payer(&self.creator.to_account_info())
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .plugins(vec![stats])
            .external_plugin_adapters(vec![transfer_oracle])
            .invoke()?;
        Ok(())
    }
}
