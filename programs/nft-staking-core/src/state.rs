use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub collection: Pubkey,
    /// Reward token base units earned per staked NFT per day.
    pub rewards_per_day: u64,
    /// Minimum seconds an NFT must stay staked before it can be unstaked.
    pub min_stake_duration: i64,
    pub rewards_bump: u8,
    pub bump: u8,
}
