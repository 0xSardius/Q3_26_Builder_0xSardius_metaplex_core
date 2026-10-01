use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{
    oracle::{seconds_since_boundary, validation_at},
    TransferOracle, CRANK_REWARD_LAMPORTS, CRANK_WINDOW_SECONDS, ORACLE_SEED, VAULT_SEED,
};

/// Permissionless crank: anyone can sync the oracle to the current on-chain time.
#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,

    #[account(mut, seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, TransferOracle>,

    #[account(mut, seeds = [VAULT_SEED], bump = oracle.vault_bump)]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl UpdateOracle<'_> {
    pub fn update_oracle(&mut self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let validation = validation_at(now);
        let flipped = validation != self.oracle.validation;
        self.oracle.validation = validation;

        // Pay only for the useful crank: the one that flips the state right after a
        // boundary. Repeat calls see no flip, so the vault can't be drained by spam.
        if !flipped || seconds_since_boundary(now) > CRANK_WINDOW_SECONDS {
            return Ok(());
        }

        // The vault is a system account, so it must stay rent-exempt or empty. Never
        // fail the crank over an underfunded vault; the oracle update matters more.
        let rent_floor = Rent::get()?.minimum_balance(0);
        let payable = self.vault.lamports().saturating_sub(rent_floor);
        if payable < CRANK_REWARD_LAMPORTS {
            msg!("vault underfunded; oracle updated without reward");
            return Ok(());
        }

        let vault_seeds: &[&[&[u8]]] = &[&[VAULT_SEED, &[self.oracle.vault_bump]]];
        transfer(
            CpiContext::new_with_signer(
                self.system_program.key(),
                Transfer {
                    from: self.vault.to_account_info(),
                    to: self.cranker.to_account_info(),
                },
                vault_seeds,
            ),
            CRANK_REWARD_LAMPORTS,
        )
    }
}
