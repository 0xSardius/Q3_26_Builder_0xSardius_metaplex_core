pub mod constants;
pub mod instructions;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;

declare_id!("814Q7NeEeZSJ3k1fDorbfKdd2tUrKCBzCFMcUeEhXYLH");

// Staking for Metaplex Core NFTs.
// The collection's update authority is a program PDA, so this program is the only
// thing that can mint into the collection or manage its authority-level plugins.

#[program]
pub mod nft_staking_core {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn create_collection(
        ctx: Context<CreateCollection>,
        name: String,
        uri: String,
    ) -> Result<()> {
        ctx.accounts.create_collection(name, uri)
    }

    #[instruction(discriminator = 1)]
    pub fn mint_nft(ctx: Context<MintNft>, name: String, uri: String) -> Result<()> {
        ctx.accounts.mint_nft(name, uri, &ctx.bumps)
    }
}
