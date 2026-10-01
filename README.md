# NFT Staking on Metaplex Core (Turbin3 Q3 2026)

Stake Metaplex Core NFTs in place. The NFT never leaves the owner's wallet: staking adds plugins to the asset that freeze it and hand the program the right to thaw or burn it. Rewards are an SPL token minted by a config PDA. Owners can claim without unstaking, or burn a staked NFT for a one-time bonus. The collection tracks how many of its assets are staked, and an Oracle plugin restricts every transfer in the collection to 09:00–17:00 UTC.

There is no stake account per NFT. Stake state lives on the asset as Core Attributes, collection stats live on the collection as Attributes, and Core itself enforces freeze, burn, and transfer rules.

Program ID: `814Q7NeEeZSJ3k1fDorbfKdd2tUrKCBzCFMcUeEhXYLH`  
Stack: Anchor 1.1.2, `mpl-core` 0.12.1, LiteSVM tests.

## Why this shape

A classic NFT staking program escrows the NFT into a vault and keeps a stake record PDA per NFT. Core makes both unnecessary:

1. **Custody → plugins.** A Core asset is one account with an `owner` field and a plugin list. A `FreezeDelegate` with `frozen: true` stops transfers and burns, and its authority can be handed to a program PDA. Owner keeps the NFT; only the program can thaw.
2. **Stake record → Attributes.** `staked_at` and `last_claimed_at` are key/value strings on the asset. No extra accounts to rent, close, or keep in sync.
3. **Global policy → collection plugins.** Plugins on the collection apply to every asset in it. A single Oracle adapter on the collection gates every transfer.

The collection's update authority is a program PDA. That is what lets the program mint into the collection and write authority-managed plugins (Attributes) on assets and on the collection.

```
create_collection → mint_nft → stake → (claim_rewards)* → unstake  → stake …
                                                        ↘ burn_staked_nft

update_oracle (crank, anyone)   09:00 → Pass      17:00 → Rejected
transfer_nft                    succeeds only while the oracle says Pass and the NFT is unstaked
```

## Accounts

| Account | Derivation / ownership | Role |
|---|---|---|
| Collection | Keypair, owned by Core | Holds `Attributes["total_staked"]` and the Oracle adapter |
| Update authority | PDA `["update_authority", collection]`, no data | Collection update authority; authority of every delegate plugin we add |
| Config | PDA `["config", collection]` | Reward rate, min stake duration. Mint authority of the rewards mint |
| Rewards mint | PDA `["rewards", config]` | SPL mint, 6 decimals |
| Asset | Keypair, owned by Core | The NFT. Stake state in its Attributes plugin |
| Oracle | PDA `["oracle"]`, global | `TransferOracle`; Core reads its Transfer result |
| Vault | PDA `["vault"]`, system-owned, no data | Lamports paid to crankers |

```rust
pub struct Config {
    pub collection: Pubkey,
    pub rewards_per_day: u64,
    pub min_stake_duration: i64,
    pub rewards_bump: u8,
    pub bump: u8,
}

pub struct TransferOracle {
    pub validation: OracleValidation, // must stay first: Core reads it at offset 8
    pub bump: u8,
    pub vault_bump: u8,
}
```

### Plugins on an asset while staked

| Plugin | Kind | Added by | Authority | Purpose |
|---|---|---|---|---|
| `Attributes` | Authority-managed | Update authority PDA | Update authority PDA | `staked_at`, `last_claimed_at` |
| `FreezeDelegate { frozen: true }` | Owner-managed | Owner | Update authority PDA | Blocks transfer and burn |
| `BurnDelegate` | Owner-managed | Owner | Update authority PDA | Lets the program burn |

Owner-managed plugins need the owner's signature to add or remove. The delegate can *use* them (thaw, burn) but cannot remove them. That is why `stake` and `unstake` are owner-signed even though the PDA does most of the work.

## Instructions

Discriminators are explicit so IDL order cannot silently shift. `2` is retired (it was `initialize_config`).

| Disc | Instruction | Signer | Effect |
|---|---|---|---|
| 0 | `create_collection` | Creator + collection keypair | Core collection with `total_staked = 0` and the Oracle adapter; config and rewards mint in the same tx. Requires the oracle to exist |
| 1 | `mint_nft` | User + asset keypair | Mint an asset into the collection, signed by the update authority PDA |
| 3 | `stake` | Owner | Write `staked_at` / `last_claimed_at`, add frozen FreezeDelegate + BurnDelegate, `total_staked += 1` |
| 4 | `unstake` | Owner | Requires `min_stake_duration`. Thaw, remove both delegates, reset `staked_at`, mint rewards, `total_staked -= 1` |
| 5 | `claim_rewards` | Owner | Mint rewards since `last_claimed_at`, move that checkpoint. Stays frozen |
| 6 | `burn_staked_nft` | Owner | Thaw, burn via BurnDelegate, mint accrued rewards + `BURN_BONUS_DAYS` of rewards, `total_staked -= 1` |
| 7 | `initialize_oracle` | Anyone | Create the global oracle from the current clock |
| 8 | `update_oracle` | Anyone | Sync the oracle to the clock; pay the cranker if it flipped near a boundary |
| 9 | `transfer_nft` | Owner | Core transfer with the oracle as a remaining account |
| 10 | `fund_vault` | Anyone | Deposit lamports for crank rewards |

| Error | When |
|---|---|
| `InvalidAsset` / `WrongCollection` / `NotOwner` | Asset fails `assert_asset` |
| `AlreadyStaked` | `stake` on a staked asset |
| `NotStaked` | `unstake` / `claim_rewards` / `burn_staked_nft` on an unstaked asset |
| `StakeLocked` | `unstake` before `min_stake_duration` |
| `NothingToClaim` | `claim_rewards` that would mint 0 |
| `CollectionStatsMissing` | Collection lacks `total_staked` (not created by this program) |

## Task 1: Core plugins

### Claim without unstaking

Two timestamps, two jobs. `staked_at` drives the unstake lock; `last_claimed_at` is where rewards accrue from. Claiming moves only the second, so it never restarts the lock and never touches the FreezeDelegate.

`rewards_since` takes `max(last_claimed_at, staked_at)`, so a stale checkpoint from an earlier stake period cannot pay out time the asset spent unstaked.

`claim_rewards` checks ownership itself. Core never sees who receives the reward tokens (the PDA signs the Attributes update), so without `assert_asset` anyone could claim someone else's rewards to their own ATA.

### Burn-to-earn

Core resolves each lifecycle event as a vote across plugins: **any Reject wins**, otherwise at least one Approve is required. A frozen FreezeDelegate rejects burn, so the BurnDelegate's Approve alone fails:

```
freeze_delegate.rs: Reject
burn_delegate.rs:   Approve
lifecycle.rs:       Reject   → error 0x9
```

`burn_staked_nft` thaws and burns in one transaction. If the burn fails, the thaw rolls back. Core leaves a 1-byte tombstone at the asset address so it cannot be reused.

### Collection stats

`total_staked` is created with the collection via `CreateCollectionV2`'s `plugins` list, and moved by `adjust_total_staked` (`UpdateCollectionPluginV1`, signed by the update authority PDA). It changes inside the same transaction as the stake / unstake / burn, so a failed instruction cannot drift the counter.

Trade-off: every stake write-locks the collection account, so stakes in one collection serialize. A high-traffic protocol would shard the counter or derive it from events.

## Task 2: Oracle plugin (time-based transfer)

### Layout

Core reads the oracle account as `OracleValidation` at an offset set on the adapter. With `ValidationResultsOffset::Anchor` it skips the 8-byte discriminator:

```
[8 discriminator][1 tag = V1][create][transfer][burn][update][bump][vault_bump]
```

`state.rs` mirrors Core's `OracleValidation` / `ExternalValidationResult` with Anchor derives (so `InitSpace` and the IDL keep working). Tests decode the account with Core's own type to prove the bytes match.

### Adapter

Added to the collection at creation:

- `lifecycle_checks`: Transfer only, `can_reject`. Core refuses oracles that try to approve (`OracleCanRejectOnly`).
- `base_address`: the oracle PDA. Core requires it in the transfer's remaining accounts; omitting it fails with `MissingExternalPluginAdapterAccount` (error 34), so the check cannot be skipped.

The crank writes **Pass** inside the window (no opinion, let owner / delegates decide) and **Rejected** outside.

### Crank incentive

Core enforces the *stored* result, not the clock. If nobody cranks at 17:00, transfers stay open. `update_oracle` therefore pays `CRANK_REWARD_LAMPORTS` from the vault, but only when:

1. the call **flips** the stored result (repeat calls cannot drain the vault), and
2. it lands within `CRANK_WINDOW_SECONDS` **after** 09:00 or 17:00.

The vault is a system account, so it must stay rent-exempt. If paying would drop it below the rent floor, the crank still updates the oracle and skips the reward. The reward never blocks the update.

### Transfer

`transfer_nft` is one CPI: Core's `TransferV1` with the oracle appended as a read-only remaining account. It does not re-check the hours. Core runs the oracle check on every transfer, including ones that bypass this program, and duplicating it would read the clock instead of the oracle and disagree whenever the oracle is stale.

## Known limits

- `initialize_oracle` is permissionless. It is one-time and its state comes from the clock, so the caller chooses nothing.
- The oracle is global: one transfer window for every collection this program creates.
- Enforcement depends on someone cranking. The reward makes that likely, not guaranteed.

## Tests

LiteSVM in [`programs/nft-staking-core/tests/mod.rs`](programs/nft-staking-core/tests/mod.rs). Clock is a sysvar, so time-based tests call `set_clock`. LiteSVM's clock starts at 0, which the program reads as "not staked"; `setup()` sets it to `START` (14:13 UTC, inside the transfer window).

The Core program is loaded from [`tests/fixtures/mpl_core.so`](tests/fixtures/mpl_core.so), dumped from mainnet:

```bash
solana program dump -u m CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d tests/fixtures/mpl_core.so
```

```
create_collection_sets_pda_update_authority
create_collection_initializes_config_and_rewards_mint
create_collection_requires_initialized_oracle
mint_nft_into_collection
mint_rejects_non_core_collection

stake_freezes_and_records_timestamp
staked_nft_cannot_be_transferred
cannot_stake_twice
non_owner_cannot_stake
unstake_before_min_duration_fails
unstake_thaws_and_pays_rewards
restake_after_unstake

claim_pays_and_keeps_nft_frozen
claim_then_unstake_pays_only_the_remainder
claim_does_not_restart_unstake_lock
claim_twice_in_same_second_fails
claim_on_unstaked_nft_fails

burn_staked_nft_pays_bonus_and_burns
burn_after_claim_only_pays_unclaimed_plus_bonus
burn_unstaked_nft_fails
non_owner_cannot_burn

total_staked_tracks_stake_unstake_and_burn

oracle_layout_is_readable_by_core
collection_carries_transfer_oracle_adapter
crank_tracks_the_window_boundaries          08:59 / 09:00 / 16:59 / 17:00 / 23:30
transfer_blocked_outside_hours_once_cranked
stale_oracle_is_what_core_enforces
transfer_without_oracle_account_fails
transfer_nft_inside_hours
transfer_nft_blocked_after_close_crank
transfer_nft_of_staked_nft_fails_even_in_hours
non_owner_cannot_transfer_nft
new_owner_can_stake_after_transfer_nft

crank_at_close_boundary_is_paid
crank_at_open_boundary_is_paid
repeat_crank_without_flip_is_unpaid
late_flip_updates_but_is_unpaid
underfunded_vault_still_updates_oracle
crank_with_empty_vault_still_updates_oracle
```

The harness `include_bytes!`s the deploy artifact, so build first:

```bash
anchor build
cargo test -p nft-staking-core --test mod
```

## Notes on `mpl-core` with Anchor 1.x

`mpl-core` 0.12.1's `anchor` feature targets Anchor 0.31/0.32. Without it, the crate builds on `solana-program` 3.x, which matches Anchor 1.x. So accounts are `UncheckedAccount` with `owner = CORE_PROGRAM_ID` / `address = CORE_PROGRAM_ID` constraints, Core is called through the generated `*CpiBuilder`s, and plugins are read with `fetch_plugin`.

`anchor build` prints stack-offset warnings from `mpl-core`'s full `Asset::deserialize`. The program never calls it on-chain (tests use it off-chain).

## Layout

```
programs/nft-staking-core/src/
  lib.rs                   instruction order only
  state.rs                 Config, TransferOracle, Core validation mirrors
  constants.rs             seeds, attribute keys, window and reward constants
  error.rs
  attributes.rs            asset checks, attribute read/write, total_staked
  rewards.rs               accrual math, mint_rewards
  oracle.rs                window math, validation_at, seconds_since_boundary
  instructions/
    create_collection.rs   collection + config + rewards mint + oracle adapter
    mint_nft.rs
    stake.rs
    unstake.rs
    claim_rewards.rs
    burn_staked_nft.rs
    initialize_oracle.rs
    update_oracle.rs       crank + reward
    transfer_nft.rs
    fund_vault.rs
tests/fixtures/mpl_core.so
```
