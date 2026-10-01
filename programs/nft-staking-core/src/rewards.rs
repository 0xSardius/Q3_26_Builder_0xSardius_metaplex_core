use anchor_lang::prelude::*;
use anchor_spl::token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface};

use crate::{error::ErrorCode, Config, CONFIG_SEED, SECONDS_PER_DAY};

pub fn accrued_rewards(rewards_per_day: u64, elapsed: i64) -> Result<u64> {
    let elapsed = u128::try_from(elapsed.max(0)).map_err(|_| error!(ErrorCode::Overflow))?;
    let amount = elapsed
        .checked_mul(rewards_per_day as u128)
        .ok_or(ErrorCode::Overflow)?
        / SECONDS_PER_DAY as u128;
    u64::try_from(amount).map_err(|_| error!(ErrorCode::Overflow))
}

/// Mints `amount` reward tokens to `to`, signed by the config PDA (the mint authority).
pub fn mint_rewards<'info>(
    token_program: &Interface<'info, TokenInterface>,
    rewards_mint: &InterfaceAccount<'info, Mint>,
    to: &InterfaceAccount<'info, TokenAccount>,
    config: &Account<'info, Config>,
    amount: u64,
) -> Result<()> {
    let config_seeds: &[&[&[u8]]] = &[&[
        CONFIG_SEED,
        config.collection.as_ref(),
        &[config.bump],
    ]];
    mint_to(
        CpiContext::new_with_signer(
            token_program.key(),
            MintTo {
                mint: rewards_mint.to_account_info(),
                to: to.to_account_info(),
                authority: config.to_account_info(),
            },
            config_seeds,
        ),
        amount,
    )
}
