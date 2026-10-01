use anchor_lang::prelude::*;

#[constant]
pub const UPDATE_AUTHORITY_SEED: &[u8] = b"update_authority";

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const REWARDS_SEED: &[u8] = b"rewards";

#[constant]
pub const ORACLE_SEED: &[u8] = b"oracle";

pub const REWARDS_DECIMALS: u8 = 6;

/// Transfers are allowed from OPEN_HOUR:00 up to (not including) CLOSE_HOUR:00 UTC.
pub const OPEN_HOUR: i64 = 9;
pub const CLOSE_HOUR: i64 = 17;
pub const SECONDS_PER_HOUR: i64 = 3_600;

pub const SECONDS_PER_DAY: i64 = 86_400;

/// Burning a staked NFT pays this many days of rewards on top of what has accrued.
pub const BURN_BONUS_DAYS: u64 = 365;

/// Attribute key on each asset: unix timestamp the stake began, or "0" when unstaked.
pub const STAKED_AT_KEY: &str = "staked_at";

/// Attribute key on each asset: unix timestamp rewards accrue from.
pub const LAST_CLAIMED_AT_KEY: &str = "last_claimed_at";

/// Attribute key on the collection: number of its assets currently staked.
pub const TOTAL_STAKED_KEY: &str = "total_staked";
