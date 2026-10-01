use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    fetch_plugin,
    instructions::UpdateCollectionPluginV1CpiBuilder,
    types::{Attribute, Attributes, Plugin, PluginType, UpdateAuthority},
};

use crate::{error::ErrorCode, LAST_CLAIMED_AT_KEY, STAKED_AT_KEY, TOTAL_STAKED_KEY};

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

fn timestamp(list: &[Attribute], key: &str) -> Option<i64> {
    get_attribute(list, key)
        .and_then(|v| v.parse::<i64>().ok())
        .filter(|ts| *ts > 0)
}

/// When the current stake began, or None when the asset isn't staked.
pub fn staked_at(list: &[Attribute]) -> Option<i64> {
    timestamp(list, STAKED_AT_KEY)
}

/// When rewards last paid out; falls back to `staked_at` if never claimed.
pub fn rewards_since(list: &[Attribute]) -> Option<i64> {
    let staked_at = staked_at(list)?;
    Some(timestamp(list, LAST_CLAIMED_AT_KEY).map_or(staked_at, |ts| ts.max(staked_at)))
}

/// Adds `delta` to the collection's "total_staked" Attribute, signed by the update authority PDA.
pub fn adjust_total_staked<'info>(
    core_program: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    update_authority: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    signer_seeds: &[&[&[u8]]],
    delta: i64,
) -> Result<()> {
    let (_, attributes, _) =
        fetch_plugin::<BaseCollectionV1, Attributes>(collection, PluginType::Attributes)
            .map_err(|_| error!(ErrorCode::CollectionStatsMissing))?;
    let mut attribute_list = attributes.attribute_list;

    let current = get_attribute(&attribute_list, TOTAL_STAKED_KEY)
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or(ErrorCode::CollectionStatsMissing)?;
    let updated = current
        .checked_add_signed(delta)
        .ok_or(ErrorCode::Overflow)?;
    set_attribute(&mut attribute_list, TOTAL_STAKED_KEY, updated.to_string());

    UpdateCollectionPluginV1CpiBuilder::new(core_program)
        .collection(collection)
        .payer(payer)
        .authority(Some(update_authority))
        .system_program(system_program)
        .plugin(Plugin::Attributes(Attributes { attribute_list }))
        .invoke_signed(signer_seeds)?;
    Ok(())
}
