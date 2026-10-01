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

/// Byte-for-byte mirror of `mpl_core::types::ExternalValidationResult`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum ValidationResult {
    Approved,
    Rejected,
    Pass,
}

/// Byte-for-byte mirror of `mpl_core::types::OracleValidation`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq, InitSpace)]
pub enum OracleValidation {
    Uninitialized,
    V1 {
        create: ValidationResult,
        transfer: ValidationResult,
        burn: ValidationResult,
        update: ValidationResult,
    },
}

/// Read by Core during Transfer. `validation` must stay the first field: the collection's
/// Oracle adapter uses `ValidationResultsOffset::Anchor`, so Core decodes it right after
/// the 8-byte discriminator.
#[account]
#[derive(InitSpace)]
pub struct TransferOracle {
    pub validation: OracleValidation,
    pub bump: u8,
    pub vault_bump: u8,
}
