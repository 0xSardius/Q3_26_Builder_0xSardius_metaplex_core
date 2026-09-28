use anchor_lang::prelude::*;
use mpl_core::{instructions::CreateV2CpiBuilder, ID as CORE_PROGRAM_ID};

use crate::UPDATE_AUTHORITY_SEED;

#[derive(Accounts)]
pub struct MintNft<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub asset: Signer<'info>,

    /// CHECK: Core validates it is a collection whose update authority is our PDA.
    #[account(mut, owner = CORE_PROGRAM_ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: PDA signer; seeds bind it to this collection.
    #[account(seeds = [UPDATE_AUTHORITY_SEED, collection.key().as_ref()], bump)]
    pub update_authority: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,

    /// CHECK: pinned to the Core program id.
    #[account(address = CORE_PROGRAM_ID)]
    pub core_program: UncheckedAccount<'info>,
}

impl MintNft<'_> {
    pub fn mint_nft(&self, name: String, uri: String, bumps: &MintNftBumps) -> Result<()> {
        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            UPDATE_AUTHORITY_SEED,
            collection_key.as_ref(),
            &[bumps.update_authority],
        ]];

        // Assets in a collection inherit the collection's update authority, so
        // `update_authority` stays None and `authority` is the collection's authority.
        CreateV2CpiBuilder::new(&self.core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(Some(&self.collection.to_account_info()))
            .authority(Some(&self.update_authority.to_account_info()))
            .payer(&self.user.to_account_info())
            .owner(Some(&self.user.to_account_info()))
            .update_authority(None)
            .system_program(&self.system_program.to_account_info())
            .name(name)
            .uri(uri)
            .invoke_signed(signer_seeds)?;
        Ok(())
    }
}
