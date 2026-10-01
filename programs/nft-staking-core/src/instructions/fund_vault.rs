use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

use crate::{TransferOracle, ORACLE_SEED, VAULT_SEED};

#[derive(Accounts)]
pub struct FundVault<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,

    #[account(seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, TransferOracle>,

    #[account(mut, seeds = [VAULT_SEED], bump = oracle.vault_bump)]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl FundVault<'_> {
    pub fn fund_vault(&self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.system_program.key(),
                Transfer {
                    from: self.funder.to_account_info(),
                    to: self.vault.to_account_info(),
                },
            ),
            amount,
        )
    }
}
