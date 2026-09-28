use anchor_lang::prelude::*;
use mpl_core::{instructions::CreateCollectionV2CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::UPDATE_AUTHORITY_SEED;

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

    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl CreateCollection<'_> {
    pub fn create_collection(&self, name: String, uri: String) -> Result<()> {
        CreateCollectionV2CpiBuilder::new(&self.core_program.to_account_info())
            .collection(&self.collection.to_account_info())
            .update_authority(Some(&self.update_authority.to_account_info()))
            .payer(&self.creator.to_account_info())
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .invoke()?;
        Ok(())
    }
}
