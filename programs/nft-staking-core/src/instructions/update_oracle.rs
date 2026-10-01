use anchor_lang::prelude::*;

use crate::{oracle::validation_at, TransferOracle, ORACLE_SEED};

/// Permissionless crank: anyone can sync the oracle to the current on-chain time.
#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    pub cranker: Signer<'info>,

    #[account(mut, seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, TransferOracle>,
}

impl UpdateOracle<'_> {
    pub fn update_oracle(&mut self) -> Result<()> {
        self.oracle.validation = validation_at(Clock::get()?.unix_timestamp);
        Ok(())
    }
}
