use event_utils::Event as _;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program::program_pack::Pack;
use solana_program_test::tokio;
use solana_sdk::instruction::AccountMeta;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use spl_token_2022::extension::metadata_pointer::instruction::initialize as initialize_metadata_pointer;
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::instruction::initialize_mint;
use spl_token_2022::state::Mint;
use spl_token_metadata_interface::instruction::initialize as initialize_token_metadata;
use test_context::test_context;

use crate::ItsTestContext;

/// Tests metadata retrieval with Metaplex metadata (fallback scenario)
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_metadata_retrieval_with_metaplex_fallback(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Deploy a local interchain token first (this creates standard mint with Metaplex metadata)
    let salt = solana_sdk::keccak::hash(b"MetaplexFallbackToken").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        "Metaplex Fallback Token".to_owned(),
        "MFT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )
    .unwrap();

    let tx = ctx
        .send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::events::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(
        deploy_event.name, "Metaplex Fallback Token",
        "token name does not match"
    );

    // Approve remote deployment
    let approve_remote_deployment =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_wallet,
            ctx.solana_wallet,
            ctx.solana_wallet,
            salt,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
        )
        .unwrap();

    ctx.send_solana_tx(&[approve_remote_deployment])
        .await
        .unwrap();

    // Deploy remote - this will use our get_token_metadata function
    let deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            ctx.solana_wallet,
            ctx.solana_wallet,
            salt,
            ctx.solana_wallet,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
            0,
        )
        .unwrap();

    let tx = ctx.send_solana_tx(&[deploy_remote_ix]).await;

    // Transaction should succeed using Metaplex fallback
    assert!(
        tx.is_ok(),
        "Expected deployment to succeed with Metaplex metadata fallback"
    );

    let tx = tx.unwrap();
    let deployment_started_event = tx
        .metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTokenDeploymentStarted::try_from_log(log).ok()
        })
        .unwrap();

    // Verify that the correct metadata was read from Metaplex
    assert_eq!(
        deployment_started_event.token_name, "Metaplex Fallback Token",
        "token name should match Metaplex metadata"
    );
    assert_eq!(
        deployment_started_event.token_symbol, "MFT",
        "token symbol should match Metaplex metadata"
    );

    Ok(())
}

/// Tests metadata retrieval with Token 2022 embedded metadata (primary scenario)
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_metadata_retrieval_with_token_2022_embedded(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a Token 2022 mint with embedded metadata - use payer as mint authority
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();

    // Calculate space needed for mint with metadata pointer extension only (metadata comes later)
    let mint_space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
            .unwrap();

    // Calculate rent for the mint account
    let mint_rent = ctx.solana_chain.fixture.get_rent(mint_space).await;

    // Create the mint account
    #[allow(clippy::disallowed_methods)]
    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &ctx.solana_wallet,
        &mint_pubkey,
        mint_rent,
        mint_space as u64,
        &spl_token_2022::id(),
    );

    // Initialize metadata pointer to point to the mint itself (embedded metadata)
    let init_metadata_pointer_ix = initialize_metadata_pointer(
        &spl_token_2022::id(),
        &mint_pubkey,
        None,              // Set authority to None to be compatible with Metaplex
        Some(mint_pubkey), // Point to the mint itself for embedded metadata
    )
    .unwrap();

    // Initialize the mint
    let init_mint_ix = initialize_mint(
        &spl_token_2022::id(),
        &mint_pubkey,
        &ctx.solana_wallet,
        Some(&ctx.solana_wallet),
        9,
    )
    .unwrap();

    // Initialize the token metadata embedded in the mint
    let init_token_metadata_ix = initialize_token_metadata(
        &spl_token_2022::id(),
        &mint_pubkey,
        &ctx.solana_wallet,
        &mint_pubkey,
        &ctx.solana_wallet,
        "Token 2022 Embedded".to_string(),
        "T22E".to_string(),
        "https://example.com/token".to_string(),
    );

    // Execute all Token-2022 setup in one transaction to ensure proper sequencing
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                create_mint_account_ix,
                init_metadata_pointer_ix,
                init_mint_ix,
                init_token_metadata_ix,
            ],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &mint_keypair,
            ],
        )
        .await
        .unwrap();

    // Register this mint as a canonical token so we can test outbound deployment
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            spl_token_2022::id(),
        )
        .unwrap();

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    // Deploy remote - this should use Token 2022 embedded metadata, not Metaplex
    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            "ethereum".to_string(),
            0,
        )
        .unwrap();

    let tx = ctx.send_solana_tx(&[deploy_remote_canonical_ix]).await;

    // Transaction should succeed using Token 2022 embedded metadata
    assert!(
        tx.is_ok(),
        "Expected deployment to succeed with Token 2022 embedded metadata: {:?}",
        tx.as_ref().err()
    );

    let tx = tx.unwrap();
    let deployment_started_event = tx
        .metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTokenDeploymentStarted::try_from_log(log).ok()
        })
        .unwrap();

    // Verify that the correct metadata was read from Token 2022 extensions, not Metaplex
    assert_eq!(
        deployment_started_event.token_name, "Token 2022 Embedded",
        "token name should match Token 2022 embedded metadata, not Metaplex"
    );
    assert_eq!(
        deployment_started_event.token_symbol, "T22E",
        "token symbol should match Token 2022 embedded metadata, not Metaplex"
    );

    Ok(())
}

/// Tests metadata retrieval with Token 2022 external metadata pointer (falls back to Metaplex)
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_metadata_retrieval_with_token_2022_external_pointer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a Token 2022 mint with metadata pointer to external account
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();

    let (metaplex_metadata_pda, _) = Metadata::find_pda(&mint_pubkey);
    let create_metaplex_ix = CreateV1Builder::new()
        .metadata(metaplex_metadata_pda)
        .mint(mint_pubkey, true)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("External Pointer Fallback".to_string())
        .symbol("EPF".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_metaplex_ix],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &mint_keypair,
            ],
        )
        .await
        .unwrap();

    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            spl_token_2022::id(),
        )
        .unwrap();

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            "ethereum".to_string(),
            0,
        )
        .unwrap();

    let tx = ctx.send_solana_tx(&[deploy_remote_canonical_ix]).await;
    let tx = tx.unwrap();
    let deployment_started_event = tx
        .metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| {
            axelar_solana_its::events::InterchainTokenDeploymentStarted::try_from_log(log).ok()
        })
        .unwrap();

    // Verify that the correct metadata was read from Metaplex fallback
    assert_eq!(
        deployment_started_event.token_name, "External Pointer Fallback",
        "token name should match Metaplex metadata used as fallback"
    );
    assert_eq!(
        deployment_started_event.token_symbol, "EPF",
        "token symbol should match Metaplex metadata used as fallback"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_metadata_retrieval_fails_no_metadata(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();

    let mint_space =
        ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::MetadataPointer])
            .unwrap();
    let mint_rent = ctx.solana_chain.fixture.get_rent(mint_space).await;

    #[allow(clippy::disallowed_methods)]
    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &ctx.solana_wallet,
        &mint_pubkey,
        mint_rent,
        mint_space as u64,
        &spl_token_2022::id(),
    );

    // Initialize metadata pointer to point to the mint itself (but don't add TokenMetadata extension)
    let init_metadata_pointer_ix = initialize_metadata_pointer(
        &spl_token_2022::id(),
        &mint_pubkey,
        None,
        Some(mint_pubkey), // Point to self, indicating embedded metadata should exist
    )
    .unwrap();

    let init_mint_ix = initialize_mint(
        &spl_token_2022::id(),
        &mint_pubkey,
        &ctx.solana_wallet,
        Some(&ctx.solana_wallet),
        9,
    )
    .unwrap();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                create_mint_account_ix,
                init_metadata_pointer_ix,
                init_mint_ix,
            ],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &mint_keypair,
            ],
        )
        .await
        .unwrap();

    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            spl_token_2022::id(),
        )
        .unwrap();

    let tx = ctx.send_solana_tx(&[register_canonical_ix]).await;
    assert!(tx.is_err());

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_metadata_retrieval_fails_wrong_mint_in_metadata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();
    let wrong_mint_keypair = Keypair::new();
    let wrong_mint_pubkey = wrong_mint_keypair.pubkey();

    let mint_space = spl_token_2022::state::Mint::LEN;
    let mint_rent = ctx.solana_chain.fixture.get_rent(mint_space).await;

    #[allow(clippy::disallowed_methods)]
    let create_mint_account_ix = solana_sdk::system_instruction::create_account(
        &ctx.solana_wallet,
        &mint_pubkey,
        mint_rent,
        mint_space as u64,
        &spl_token_2022::id(),
    );

    let init_mint_ix = initialize_mint(
        &spl_token_2022::id(),
        &mint_pubkey,
        &ctx.solana_wallet,
        Some(&ctx.solana_wallet),
        9,
    )
    .unwrap();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_mint_account_ix, init_mint_ix],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &mint_keypair,
            ],
        )
        .await
        .unwrap();

    let (wrong_mint_metadata_pda, _) = Metadata::find_pda(&wrong_mint_pubkey);
    let create_metaplex_ix = CreateV1Builder::new()
        .metadata(wrong_mint_metadata_pda)
        .mint(wrong_mint_pubkey, true)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Wrong Mint Reference".to_string())
        .symbol("WMR".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[create_metaplex_ix],
            &[
                &ctx.solana_chain.fixture.payer.insecure_clone(),
                &wrong_mint_keypair,
            ],
        )
        .await
        .unwrap();

    let mut register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            mint_pubkey,
            spl_token_2022::id(),
        )
        .unwrap();
    register_canonical_ix.accounts[1] = AccountMeta::new_readonly(wrong_mint_metadata_pda, false);

    let tx = ctx.send_solana_tx(&[register_canonical_ix]).await;
    assert!(tx.is_err());

    Ok(())
}
