use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::{
    instructions::CreateCollectionV2CpiBuilder,
    types::{
        Attribute, Attributes, ExternalPluginAdapterInitInfo, HookableLifecycleEvent,
        OracleInitInfo, Plugin, PluginAuthorityPair, ValidationResultsOffset,
    },
    ExternalCheckResultBits, ID as CORE_PROGRAM_ID,
};

use crate::{
    Config, TransferOracle, CONFIG_SEED, ORACLE_SEED, REWARDS_DECIMALS, REWARDS_SEED,
    TOTAL_STAKED_KEY, UPDATE_AUTHORITY_SEED,
};

/// Creates the collection together with its staking config and rewards mint, so no one
/// else can initialize the config first and pick the reward terms.
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

    #[account(
        init,
        payer = creator,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = creator,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump,
        mint::decimals = REWARDS_DECIMALS,
        mint::authority = config,
        mint::token_program = token_program
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    /// Must already exist: the collection's Transfer checks fail without it.
    #[account(seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, TransferOracle>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl CreateCollection<'_> {
    pub fn create_collection(
        &mut self,
        name: String,
        uri: String,
        rewards_per_day: u64,
        min_stake_duration: i64,
        bumps: &CreateCollectionBumps,
    ) -> Result<()> {
        self.config.set_inner(Config {
            collection: self.collection.key(),
            rewards_per_day,
            min_stake_duration,
            rewards_bump: bumps.rewards_mint,
            bump: bumps.config,
        });

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
