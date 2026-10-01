use anchor_lang::prelude::*;

#[constant]
pub const UPDATE_AUTHORITY_SEED: &[u8] = b"update_authority";

#[constant]
pub const CONFIG_SEED: &[u8] = b"config";

#[constant]
pub const REWARDS_SEED: &[u8] = b"rewards";

pub const REWARDS_DECIMALS: u8 = 6;

pub const SECONDS_PER_DAY: i64 = 86_400;

/// Attribute key on each asset: unix timestamp rewards accrue from, or "0" when unstaked.
pub const STAKED_AT_KEY: &str = "staked_at";
