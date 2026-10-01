use {
    anchor_lang::{
        solana_program::{instruction::Instruction, program_pack::Pack},
        system_program::ID as SYSTEM_PROGRAM_ID,
        InstructionData, ToAccountMetas,
    },
    anchor_spl::{
        associated_token::{get_associated_token_address, ID as ASSOCIATED_TOKEN_PROGRAM_ID},
        token::{spl_token, ID as TOKEN_PROGRAM_ID},
    },
    litesvm::LiteSVM,
    mpl_core::{
        accounts::{BaseAssetV1, BaseCollectionV1},
        instructions::TransferV1Builder,
        types::UpdateAuthority,
        Asset, ID as CORE_PROGRAM_ID,
    },
    nft_staking_core::{CONFIG_SEED, REWARDS_SEED, STAKED_AT_KEY, UPDATE_AUTHORITY_SEED},
    solana_clock::Clock,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

const START: i64 = 1_790_000_000;
const DAY: i64 = 86_400;
const REWARDS_PER_DAY: u64 = 10_000_000;
const MIN_STAKE_DURATION: i64 = DAY;

struct Fixture {
    collection: Pubkey,
    update_authority: Pubkey,
    config: Pubkey,
    rewards_mint: Pubkey,
}

fn setup() -> (LiteSVM, Keypair) {
    let payer = Keypair::new();
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/nft_staking_core.so"
    ));
    svm.add_program(nft_staking_core::id(), bytes).unwrap();

    // Dumped from mainnet: `solana program dump -u m CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d tests/fixtures/mpl_core.so`
    let core = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/fixtures/mpl_core.so"
    ))
    .expect("missing tests/fixtures/mpl_core.so");
    svm.add_program(CORE_PROGRAM_ID, &core).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    // LiteSVM's clock starts at 0, which the program reads as "not staked".
    set_clock(&mut svm, START);
    (svm, payer)
}

fn send(svm: &mut LiteSVM, payer: &Keypair, extra: &[&Keypair], ix: Instruction) {
    try_send(svm, payer, extra, ix).unwrap();
}

fn try_send(
    svm: &mut LiteSVM,
    payer: &Keypair,
    extra: &[&Keypair],
    ix: Instruction,
) -> litesvm::types::TransactionResult {
    let message = Message::new(&[ix], Some(&payer.pubkey()));
    let mut signers = vec![payer];
    signers.extend_from_slice(extra);
    let transaction = Transaction::new(&signers, message, svm.latest_blockhash());
    let result = svm.send_transaction(transaction);
    svm.expire_blockhash();
    result
}

fn set_clock(svm: &mut LiteSVM, unix_timestamp: i64) {
    let mut clock = svm.get_sysvar::<Clock>();
    clock.unix_timestamp = unix_timestamp;
    svm.set_sysvar(&clock);
}

fn create_collection(svm: &mut LiteSVM, payer: &Keypair) -> Fixture {
    let collection = Keypair::new();
    let program_id = nft_staking_core::id();
    let update_authority = Pubkey::find_program_address(
        &[UPDATE_AUTHORITY_SEED, collection.pubkey().as_ref()],
        &program_id,
    )
    .0;
    let config =
        Pubkey::find_program_address(&[CONFIG_SEED, collection.pubkey().as_ref()], &program_id).0;
    let rewards_mint =
        Pubkey::find_program_address(&[REWARDS_SEED, config.as_ref()], &program_id).0;

    send(
        svm,
        payer,
        &[&collection],
        Instruction {
            program_id,
            accounts: nft_staking_core::accounts::CreateCollection {
                creator: payer.pubkey(),
                collection: collection.pubkey(),
                update_authority,
                system_program: SYSTEM_PROGRAM_ID,
                core_program: CORE_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: nft_staking_core::instruction::CreateCollection {
                name: "Turbin3 Stakers".to_string(),
                uri: "https://example.com/collection.json".to_string(),
            }
            .data(),
        },
    );

    Fixture {
        collection: collection.pubkey(),
        update_authority,
        config,
        rewards_mint,
    }
}

fn initialize_config(svm: &mut LiteSVM, payer: &Keypair, fx: &Fixture) {
    send(
        svm,
        payer,
        &[],
        Instruction {
            program_id: nft_staking_core::id(),
            accounts: nft_staking_core::accounts::InitializeConfig {
                admin: payer.pubkey(),
                collection: fx.collection,
                config: fx.config,
                rewards_mint: fx.rewards_mint,
                token_program: TOKEN_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: nft_staking_core::instruction::InitializeConfig {
                rewards_per_day: REWARDS_PER_DAY,
                min_stake_duration: MIN_STAKE_DURATION,
            }
            .data(),
        },
    );
}

fn mint_nft(svm: &mut LiteSVM, user: &Keypair, fx: &Fixture) -> Pubkey {
    let asset = Keypair::new();
    send(
        svm,
        user,
        &[&asset],
        Instruction {
            program_id: nft_staking_core::id(),
            accounts: nft_staking_core::accounts::MintNft {
                user: user.pubkey(),
                asset: asset.pubkey(),
                collection: fx.collection,
                update_authority: fx.update_authority,
                system_program: SYSTEM_PROGRAM_ID,
                core_program: CORE_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: nft_staking_core::instruction::MintNft {
                name: "Staker #1".to_string(),
                uri: "https://example.com/1.json".to_string(),
            }
            .data(),
        },
    );
    asset.pubkey()
}

/// Collection with config initialized and one NFT minted to `payer`.
fn staking_setup() -> (LiteSVM, Keypair, Fixture, Pubkey) {
    let (mut svm, payer) = setup();
    let fx = create_collection(&mut svm, &payer);
    initialize_config(&mut svm, &payer, &fx);
    let asset = mint_nft(&mut svm, &payer, &fx);
    (svm, payer, fx, asset)
}

fn stake_ix(owner: &Pubkey, fx: &Fixture, asset: &Pubkey) -> Instruction {
    Instruction {
        program_id: nft_staking_core::id(),
        accounts: nft_staking_core::accounts::Stake {
            owner: *owner,
            asset: *asset,
            collection: fx.collection,
            update_authority: fx.update_authority,
            config: fx.config,
            system_program: SYSTEM_PROGRAM_ID,
            core_program: CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: nft_staking_core::instruction::Stake {}.data(),
    }
}

fn unstake_ix(owner: &Pubkey, fx: &Fixture, asset: &Pubkey) -> Instruction {
    Instruction {
        program_id: nft_staking_core::id(),
        accounts: nft_staking_core::accounts::Unstake {
            owner: *owner,
            asset: *asset,
            collection: fx.collection,
            update_authority: fx.update_authority,
            config: fx.config,
            rewards_mint: fx.rewards_mint,
            owner_rewards_ata: get_associated_token_address(owner, &fx.rewards_mint),
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
            core_program: CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: nft_staking_core::instruction::Unstake {}.data(),
    }
}

fn claim_ix(owner: &Pubkey, fx: &Fixture, asset: &Pubkey) -> Instruction {
    Instruction {
        program_id: nft_staking_core::id(),
        accounts: nft_staking_core::accounts::ClaimRewards {
            owner: *owner,
            asset: *asset,
            collection: fx.collection,
            update_authority: fx.update_authority,
            config: fx.config,
            rewards_mint: fx.rewards_mint,
            owner_rewards_ata: get_associated_token_address(owner, &fx.rewards_mint),
            token_program: TOKEN_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: SYSTEM_PROGRAM_ID,
            core_program: CORE_PROGRAM_ID,
        }
        .to_account_metas(None),
        data: nft_staking_core::instruction::ClaimRewards {}.data(),
    }
}

fn transfer_ix(owner: &Pubkey, fx: &Fixture, asset: &Pubkey, new_owner: &Pubkey) -> Instruction {
    TransferV1Builder::new()
        .asset(*asset)
        .collection(Some(fx.collection))
        .payer(*owner)
        .new_owner(*new_owner)
        .instruction()
}

fn asset_data(svm: &LiteSVM, asset: &Pubkey) -> BaseAssetV1 {
    let account = svm.get_account(asset).unwrap();
    BaseAssetV1::from_bytes(&account.data).unwrap()
}

fn full_asset(svm: &LiteSVM, asset: &Pubkey) -> Box<Asset> {
    let account = svm.get_account(asset).unwrap();
    Asset::deserialize(&account.data).unwrap()
}

fn staked_at_attr(svm: &LiteSVM, asset: &Pubkey) -> Option<String> {
    full_asset(svm, asset)
        .plugin_list
        .attributes?
        .attributes
        .attribute_list
        .into_iter()
        .find(|a| a.key == STAKED_AT_KEY)
        .map(|a| a.value)
}

fn is_frozen(svm: &LiteSVM, asset: &Pubkey) -> bool {
    full_asset(svm, asset)
        .plugin_list
        .freeze_delegate
        .is_some_and(|p| p.freeze_delegate.frozen)
}

fn collection_data(svm: &LiteSVM, collection: &Pubkey) -> BaseCollectionV1 {
    let account = svm.get_account(collection).unwrap();
    BaseCollectionV1::from_bytes(&account.data).unwrap()
}

fn token_amount(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata).unwrap();
    spl_token::state::Account::unpack(&account.data)
        .unwrap()
        .amount
}

#[test]
fn create_collection_sets_pda_update_authority() {
    let (mut svm, payer) = setup();
    let fx = create_collection(&mut svm, &payer);

    let collection = collection_data(&svm, &fx.collection);
    assert_eq!(collection.update_authority, fx.update_authority);
    assert_eq!(collection.num_minted, 0);
}

#[test]
fn mint_nft_into_collection() {
    let (mut svm, payer) = setup();
    let fx = create_collection(&mut svm, &payer);
    let asset = mint_nft(&mut svm, &payer, &fx);

    let asset = asset_data(&svm, &asset);
    assert_eq!(asset.owner, payer.pubkey());
    assert_eq!(
        asset.update_authority,
        UpdateAuthority::Collection(fx.collection)
    );

    let collection = collection_data(&svm, &fx.collection);
    assert_eq!(collection.num_minted, 1);
    assert_eq!(collection.current_size, 1);
}

#[test]
fn mint_rejects_non_core_collection() {
    let (mut svm, payer) = setup();
    let fx = create_collection(&mut svm, &payer);
    let fake = Fixture {
        collection: Keypair::new().pubkey(),
        ..fx
    };
    let asset = Keypair::new();

    let result = try_send(
        &mut svm,
        &payer,
        &[&asset],
        Instruction {
            program_id: nft_staking_core::id(),
            accounts: nft_staking_core::accounts::MintNft {
                user: payer.pubkey(),
                asset: asset.pubkey(),
                collection: fake.collection,
                update_authority: fake.update_authority,
                system_program: SYSTEM_PROGRAM_ID,
                core_program: CORE_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: nft_staking_core::instruction::MintNft {
                name: "Nope".to_string(),
                uri: "https://example.com/x.json".to_string(),
            }
            .data(),
        },
    );
    assert!(result.is_err());
}

#[test]
fn stake_freezes_and_records_timestamp() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    assert!(is_frozen(&svm, &asset));
    assert_eq!(staked_at_attr(&svm, &asset), Some(START.to_string()));
    // Owner still owns it; staking is in-place, no custody transfer.
    assert_eq!(asset_data(&svm, &asset).owner, payer.pubkey());
}

#[test]
fn staked_nft_cannot_be_transferred() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    let recipient = Keypair::new().pubkey();
    let result = try_send(
        &mut svm,
        &payer,
        &[],
        transfer_ix(&payer.pubkey(), &fx, &asset, &recipient),
    );
    assert!(result.is_err());
    assert_eq!(asset_data(&svm, &asset).owner, payer.pubkey());
}

#[test]
fn cannot_stake_twice() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));
    let result = try_send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));
    assert!(result.is_err());
}

#[test]
fn non_owner_cannot_stake() {
    let (mut svm, _payer, fx, asset) = staking_setup();
    let stranger = Keypair::new();
    svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let result = try_send(
        &mut svm,
        &stranger,
        &[],
        stake_ix(&stranger.pubkey(), &fx, &asset),
    );
    assert!(result.is_err());
}

#[test]
fn unstake_before_min_duration_fails() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + MIN_STAKE_DURATION - 1);
    let result = try_send(&mut svm, &payer, &[], unstake_ix(&payer.pubkey(), &fx, &asset));
    assert!(result.is_err());
    assert!(is_frozen(&svm, &asset));
}

#[test]
fn unstake_thaws_and_pays_rewards() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + 2 * DAY);
    send(&mut svm, &payer, &[], unstake_ix(&payer.pubkey(), &fx, &asset));

    assert!(full_asset(&svm, &asset).plugin_list.freeze_delegate.is_none());
    assert_eq!(staked_at_attr(&svm, &asset), Some("0".to_string()));
    let ata = get_associated_token_address(&payer.pubkey(), &fx.rewards_mint);
    assert_eq!(token_amount(&svm, &ata), 2 * REWARDS_PER_DAY);

    // Transferable again once unstaked.
    let recipient = Keypair::new().pubkey();
    send(
        &mut svm,
        &payer,
        &[],
        transfer_ix(&payer.pubkey(), &fx, &asset, &recipient),
    );
    assert_eq!(asset_data(&svm, &asset).owner, recipient);
}

#[test]
fn restake_after_unstake() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));
    set_clock(&mut svm, START + DAY);
    send(&mut svm, &payer, &[], unstake_ix(&payer.pubkey(), &fx, &asset));

    // Second stake hits the UpdatePlugin path since Attributes already exists.
    set_clock(&mut svm, START + 3 * DAY);
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));
    assert!(is_frozen(&svm, &asset));
    assert_eq!(
        staked_at_attr(&svm, &asset),
        Some((START + 3 * DAY).to_string())
    );
}

#[test]
fn claim_pays_and_keeps_nft_frozen() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + DAY);
    send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));

    let ata = get_associated_token_address(&payer.pubkey(), &fx.rewards_mint);
    assert_eq!(token_amount(&svm, &ata), REWARDS_PER_DAY);
    assert!(is_frozen(&svm, &asset));
    assert_eq!(staked_at_attr(&svm, &asset), Some(START.to_string()));

    let recipient = Keypair::new().pubkey();
    let result = try_send(
        &mut svm,
        &payer,
        &[],
        transfer_ix(&payer.pubkey(), &fx, &asset, &recipient),
    );
    assert!(result.is_err());
}

#[test]
fn claim_then_unstake_pays_only_the_remainder() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + DAY);
    send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));
    set_clock(&mut svm, START + 3 * DAY);
    send(&mut svm, &payer, &[], unstake_ix(&payer.pubkey(), &fx, &asset));

    let ata = get_associated_token_address(&payer.pubkey(), &fx.rewards_mint);
    assert_eq!(token_amount(&svm, &ata), 3 * REWARDS_PER_DAY);
}

#[test]
fn claim_does_not_restart_unstake_lock() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + MIN_STAKE_DURATION - 10);
    send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));
    set_clock(&mut svm, START + MIN_STAKE_DURATION);
    send(&mut svm, &payer, &[], unstake_ix(&payer.pubkey(), &fx, &asset));

    assert!(full_asset(&svm, &asset).plugin_list.freeze_delegate.is_none());
}

#[test]
fn claim_twice_in_same_second_fails() {
    let (mut svm, payer, fx, asset) = staking_setup();
    send(&mut svm, &payer, &[], stake_ix(&payer.pubkey(), &fx, &asset));

    set_clock(&mut svm, START + DAY);
    send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));
    let result = try_send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));
    assert!(result.is_err());
}

#[test]
fn claim_on_unstaked_nft_fails() {
    let (mut svm, payer, fx, asset) = staking_setup();
    let result = try_send(&mut svm, &payer, &[], claim_ix(&payer.pubkey(), &fx, &asset));
    assert!(result.is_err());
}
