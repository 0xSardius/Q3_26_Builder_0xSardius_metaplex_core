use {
    anchor_lang::{
        solana_program::instruction::Instruction, system_program::ID as SYSTEM_PROGRAM_ID,
        InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    mpl_core::{
        accounts::{BaseAssetV1, BaseCollectionV1},
        types::UpdateAuthority,
        ID as CORE_PROGRAM_ID,
    },
    nft_staking_core::UPDATE_AUTHORITY_SEED,
    solana_keypair::Keypair,
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_signer::Signer,
    solana_transaction::Transaction,
};

struct Fixture {
    collection: Pubkey,
    update_authority: Pubkey,
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
    svm.send_transaction(transaction)
}

fn create_collection(svm: &mut LiteSVM, payer: &Keypair) -> Fixture {
    let collection = Keypair::new();
    let update_authority = Pubkey::find_program_address(
        &[UPDATE_AUTHORITY_SEED, collection.pubkey().as_ref()],
        &nft_staking_core::id(),
    )
    .0;

    send(
        svm,
        payer,
        &[&collection],
        Instruction {
            program_id: nft_staking_core::id(),
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
    }
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

fn asset_data(svm: &LiteSVM, asset: &Pubkey) -> BaseAssetV1 {
    let account = svm.get_account(asset).unwrap();
    BaseAssetV1::from_bytes(&account.data).unwrap()
}

fn collection_data(svm: &LiteSVM, collection: &Pubkey) -> BaseCollectionV1 {
    let account = svm.get_account(collection).unwrap();
    BaseCollectionV1::from_bytes(&account.data).unwrap()
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
        update_authority: fx.update_authority,
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
