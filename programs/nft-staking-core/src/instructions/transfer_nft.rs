use anchor_lang::prelude::*;
use mpl_core::{instructions::TransferV1CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::{TransferOracle, ORACLE_SEED};

#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    /// CHECK: Core-owned; Core verifies `owner` is the asset's owner.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub asset: UncheckedAccount<'info>,

    /// CHECK: Core-owned; Core verifies the asset belongs to it.
    #[account(owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: any address can receive an asset.
    pub new_owner: UncheckedAccount<'info>,

    #[account(seeds = [ORACLE_SEED], bump = oracle.bump)]
    pub oracle: Account<'info, TransferOracle>,

    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl TransferNft<'_> {
    pub fn transfer_nft(&self) -> Result<()> {
        // The hours rule lives in Core's lifecycle check, not here. Our job is only to
        // hand Core the oracle account so the collection's adapter can read it.
        TransferV1CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .payer(&self.owner.to_account_info())
            .authority(Some(&self.owner.to_account_info()))
            .new_owner(&self.new_owner.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .add_remaining_account(&self.oracle.to_account_info(), false, false)
            .invoke()?;
        Ok(())
    }
}
