use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseAssetV1,
    fetch_plugin,
    types::{Attribute, Attributes, PluginType, UpdateAuthority},
};

use crate::{error::ErrorCode, SECONDS_PER_DAY, STAKED_AT_KEY};

/// Checks the asset belongs to `collection` and is owned by `owner`.
pub fn assert_asset(asset: &AccountInfo, collection: &Pubkey, owner: &Pubkey) -> Result<()> {
    let base = BaseAssetV1::from_bytes(&asset.try_borrow_data()?)
        .map_err(|_| error!(ErrorCode::InvalidAsset))?;
    require!(
        base.update_authority == UpdateAuthority::Collection(*collection),
        ErrorCode::WrongCollection
    );
    require_keys_eq!(base.owner, *owner, ErrorCode::NotOwner);
    Ok(())
}

/// The asset's attribute list, or None if it has no Attributes plugin yet.
pub fn asset_attributes(asset: &AccountInfo) -> Option<Vec<Attribute>> {
    fetch_plugin::<BaseAssetV1, Attributes>(asset, PluginType::Attributes)
        .ok()
        .map(|(_, attributes, _)| attributes.attribute_list)
}

pub fn get_attribute<'a>(list: &'a [Attribute], key: &str) -> Option<&'a str> {
    list.iter()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

pub fn set_attribute(list: &mut Vec<Attribute>, key: &str, value: String) {
    match list.iter_mut().find(|a| a.key == key) {
        Some(attribute) => attribute.value = value,
        None => list.push(Attribute {
            key: key.to_string(),
            value,
        }),
    }
}

/// Timestamp rewards accrue from, or None when the asset isn't staked.
pub fn staked_at(list: &[Attribute]) -> Option<i64> {
    get_attribute(list, STAKED_AT_KEY)
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|ts| *ts > 0)
}

pub fn accrued_rewards(rewards_per_day: u64, elapsed: i64) -> Result<u64> {
    let elapsed = u128::try_from(elapsed.max(0)).map_err(|_| error!(ErrorCode::Overflow))?;
    let amount = elapsed
        .checked_mul(rewards_per_day as u128)
        .ok_or(ErrorCode::Overflow)?
        / SECONDS_PER_DAY as u128;
    u64::try_from(amount).map_err(|_| error!(ErrorCode::Overflow))
}
