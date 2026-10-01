use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Account is not a valid Core asset")]
    InvalidAsset,
    #[msg("Asset does not belong to this collection")]
    WrongCollection,
    #[msg("Signer does not own the asset")]
    NotOwner,
    #[msg("Asset is already staked")]
    AlreadyStaked,
    #[msg("Asset is not staked")]
    NotStaked,
    #[msg("Minimum stake duration has not elapsed")]
    StakeLocked,
    #[msg("Arithmetic overflow")]
    Overflow,
}
