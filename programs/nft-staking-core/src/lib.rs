pub mod attributes;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod rewards;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("814Q7NeEeZSJ3k1fDorbfKdd2tUrKCBzCFMcUeEhXYLH");

// Staking for Metaplex Core NFTs.
// The collection's update authority is a program PDA, so this program is the only
// thing that can mint into the collection or manage its authority-level plugins.
//
// stake:   Attributes["staked_at"] = now, FreezeDelegate(frozen) with the PDA as authority
// unstake: thaw + remove FreezeDelegate, reset "staked_at", mint accrued rewards
// claim:   mint rewards since "last_claimed_at", move that checkpoint; stays frozen

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

    #[instruction(discriminator = 2)]
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        rewards_per_day: u64,
        min_stake_duration: i64,
    ) -> Result<()> {
        ctx.accounts
            .initialize_config(rewards_per_day, min_stake_duration, &ctx.bumps)
    }

    #[instruction(discriminator = 3)]
    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake(&ctx.bumps)
    }

    #[instruction(discriminator = 4)]
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake(&ctx.bumps)
    }

    #[instruction(discriminator = 5)]
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.claim_rewards(&ctx.bumps)
    }
}
