use anchor_lang::prelude::*;
use anchor_spl::token_interface::{Mint, TokenInterface};
use mpl_core::ID as CORE_PROGRAM_ID;

use crate::{Config, CONFIG_SEED, REWARDS_DECIMALS, REWARDS_SEED};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    /// CHECK: must be a Core-owned account; used only as a seed.
    #[account(owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    #[account(
        init,
        payer = admin,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        space = Config::DISCRIMINATOR.len() + Config::INIT_SPACE,
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump,
        mint::decimals = REWARDS_DECIMALS,
        mint::authority = config,
        mint::token_program = token_program
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

impl InitializeConfig<'_> {
    pub fn initialize_config(
        &mut self,
        rewards_per_day: u64,
        min_stake_duration: i64,
        bumps: &InitializeConfigBumps,
    ) -> Result<()> {
        self.config.set_inner(Config {
            collection: self.collection.key(),
            rewards_per_day,
            min_stake_duration,
            rewards_bump: bumps.rewards_mint,
            bump: bumps.config,
        });
        Ok(())
    }
}
