pub mod attributes;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod oracle;
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
// stake:   Attributes["staked_at"] = now, FreezeDelegate(frozen) + BurnDelegate with the PDA as authority
// unstake: thaw + remove both delegates, reset "staked_at", mint accrued rewards
// claim:   mint rewards since "last_claimed_at", move that checkpoint; stays frozen
// burn:    thaw, burn via BurnDelegate, mint accrued rewards + a one-time bonus
// The collection's own Attributes["total_staked"] goes +1 on stake, -1 on unstake and burn.
//
// Transfers: the collection carries an Oracle adapter pointing at the global oracle PDA.
// A permissionless crank writes Pass (09:00-17:00 UTC) or Rejected into it for Transfer.
// transfer_nft forwards the oracle to Core as a remaining account; Core enforces the result.
// The crank is paid from a vault PDA only when it flips the state within 5 min of a boundary.

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

    #[instruction(discriminator = 6)]
    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.burn_staked_nft(&ctx.bumps)
    }

    #[instruction(discriminator = 7)]
    pub fn initialize_oracle(ctx: Context<InitializeOracle>) -> Result<()> {
        ctx.accounts.initialize_oracle(&ctx.bumps)
    }

    #[instruction(discriminator = 8)]
    pub fn update_oracle(ctx: Context<UpdateOracle>) -> Result<()> {
        ctx.accounts.update_oracle()
    }

    #[instruction(discriminator = 9)]
    pub fn transfer_nft(ctx: Context<TransferNft>) -> Result<()> {
        ctx.accounts.transfer_nft()
    }

    #[instruction(discriminator = 10)]
    pub fn fund_vault(ctx: Context<FundVault>, amount: u64) -> Result<()> {
        ctx.accounts.fund_vault(amount)
    }
}
