use anchor_lang::prelude::*;

use crate::{oracle::validation_at, TransferOracle, ORACLE_SEED, VAULT_SEED};

#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        init,
        payer = payer,
        seeds = [ORACLE_SEED],
        space = TransferOracle::DISCRIMINATOR.len() + TransferOracle::INIT_SPACE,
        bump
    )]
    pub oracle: Account<'info, TransferOracle>,

    /// Data-less, system-owned PDA holding crank rewards; funded via `fund_vault`.
    #[account(seeds = [VAULT_SEED], bump)]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl InitializeOracle<'_> {
    pub fn initialize_oracle(&mut self, bumps: &InitializeOracleBumps) -> Result<()> {
        self.oracle.set_inner(TransferOracle {
            validation: validation_at(Clock::get()?.unix_timestamp),
            bump: bumps.oracle,
            vault_bump: bumps.vault,
        });
        Ok(())
    }
}
