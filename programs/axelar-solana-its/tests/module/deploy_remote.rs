use event_utils::Event as _;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::instruction::Instruction;
use test_context::test_context;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_interchain_token_with_valid_metadata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Deploy a local interchain token first
    let salt = solana_sdk::keccak::hash(b"ValidMetadataToken").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Valid Metadata Token".to_owned(),
        "VMT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    let tx = ctx
        .send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let deploy_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| axelar_solana_its::event::InterchainTokenDeployed::try_from_log(log).ok())
        .unwrap();

    assert_eq!(
        deploy_event.name, "Valid Metadata Token",
        "token name does not match"
    );

    // Approve remote deployment
    let approve_remote_deployment =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_wallet,
            ctx.solana_wallet,
            salt,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
        )?;

    ctx.send_solana_tx(&[approve_remote_deployment])
        .await
        .unwrap();

    // Deploy remote with correct mint and metadata accounts
    let deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            ctx.solana_wallet,
            salt,
            ctx.solana_wallet,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
            0,
        )?;

    let tx = ctx.send_solana_tx(&[deploy_remote_ix]).await;

    // Transaction should succeed
    assert!(
        tx.is_ok(),
        "Expected deployment to succeed with valid metadata"
    );

    let tx = tx.unwrap();
    let deployment_started_event = tx
        .metadata
        .unwrap()
        .log_messages
        .iter()
        .find_map(|log| {
            axelar_solana_its::event::InterchainTokenDeploymentStarted::try_from_log(log).ok()
        })
        .unwrap();

    assert_eq!(
        deployment_started_event.token_name, "Valid Metadata Token",
        "token name does not match"
    );
    assert_eq!(
        deployment_started_event.token_symbol, "VMT",
        "token symbol does not match"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_interchain_token_with_mismatched_metadata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // First, create a separate mint that we'll use for the mismatched metadata test
    let separate_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 8)
        .await;

    // Create metadata for the separate mint
    let (separate_metadata_pda, _) = Metadata::find_pda(&separate_mint);

    let create_metadata_ix = CreateV1Builder::new()
        .metadata(separate_metadata_pda)
        .mint(separate_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Separate Token".to_string())
        .symbol("SEP".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(8)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Now deploy a local interchain token
    let salt = solana_sdk::keccak::hash(b"MismatchedMetadataToken").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Mismatched Token".to_owned(),
        "MMT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    // Get the metadata account for the actual interchain token
    let (interchain_token_metadata_pda, _) = Metadata::find_pda(&interchain_token_pda);

    // Approve remote deployment
    let approve_remote_deployment =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_wallet,
            ctx.solana_wallet,
            salt,
            "ethereum".to_string(),
            vec![5, 6, 7, 8],
        )?;

    ctx.send_solana_tx(&[approve_remote_deployment])
        .await
        .unwrap();

    // Try to deploy remote with mismatched mint and metadata
    let deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            ctx.solana_wallet,
            salt,
            ctx.solana_wallet,
            "ethereum".to_string(),
            vec![5, 6, 7, 8],
            0,
        )?;

    // Get the accounts from the instruction
    let mut accounts = deploy_remote_ix.accounts.clone();

    // Temper with the accounts
    accounts[1].pubkey = separate_mint;
    accounts[2].pubkey = interchain_token_metadata_pda;

    // Create the modified instruction
    let mismatched_ix = Instruction {
        program_id: axelar_solana_its::id(),
        accounts,
        data: deploy_remote_ix.data,
    };

    let result = ctx.send_solana_tx(&[mismatched_ix]).await;

    // Transaction should fail
    assert!(
        result.is_err(),
        "Expected deployment to fail with mismatched metadata"
    );

    let err = result.unwrap_err();
    let error_logs = err.metadata.unwrap().log_messages;

    // Check for the specific error message
    let has_invalid_mint_error = error_logs
        .iter()
        .any(|log| log.contains("The metadata and mint accounts passed don't match"));

    assert!(
        has_invalid_mint_error,
        "Expected 'The metadata and mint accounts passed don't match' error"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_canonical_token_with_mismatched_metadata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create two separate mints to test the canonical token deployment
    let canonical_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let other_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create metadata for canonical mint
    let (canonical_metadata_pda, _) = Metadata::find_pda(&canonical_mint);

    let create_canonical_metadata_ix = CreateV1Builder::new()
        .metadata(canonical_metadata_pda)
        .mint(canonical_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Canonical Token".to_string())
        .symbol("CAN".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_canonical_metadata_ix])
        .await
        .unwrap();

    // Create metadata for other mint
    let (other_metadata_pda, _) = Metadata::find_pda(&other_mint);

    let create_other_metadata_ix = CreateV1Builder::new()
        .metadata(other_metadata_pda)
        .mint(other_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Other Token".to_string())
        .symbol("OTH".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_other_metadata_ix])
        .await
        .unwrap();

    // Register the canonical token
    let register_canonical_ix =
        axelar_solana_its::instruction::register_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[register_canonical_ix]).await.unwrap();

    // Try to deploy remote canonical token with mismatched mint and metadata
    let deploy_remote_canonical_ix =
        axelar_solana_its::instruction::deploy_remote_canonical_interchain_token(
            ctx.solana_wallet,
            canonical_mint,
            "ethereum".to_string(),
            0,
        )?;

    // Get the accounts from the instruction
    let mut accounts = deploy_remote_canonical_ix.accounts.clone();

    // Replace the mint account (position 1) with the other mint
    // but keep the metadata account pointing to the canonical mint's metadata
    accounts[1].pubkey = other_mint;

    // Create the modified instruction
    let mismatched_canonical_ix = Instruction {
        program_id: axelar_solana_its::id(),
        accounts,
        data: deploy_remote_canonical_ix.data,
    };

    let result = ctx.send_solana_tx(&[mismatched_canonical_ix]).await;

    // Transaction should fail
    assert!(
        result.is_err(),
        "Expected canonical deployment to fail with mismatched metadata"
    );

    let err = result.unwrap_err();
    let error_logs = err.metadata.unwrap().log_messages;

    // Check for the specific error message
    let has_invalid_mint_error = error_logs
        .iter()
        .any(|log| log.contains("The metadata and mint accounts passed don't match"));

    assert!(
        has_invalid_mint_error,
        "Expected 'The metadata and mint accounts passed don't match' error for canonical token"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_without_minter_with_mismatched_metadata(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Create a separate mint
    let separate_mint = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    // Create metadata for the separate mint
    let (separate_metadata_pda, _) = Metadata::find_pda(&separate_mint);

    let create_metadata_ix = CreateV1Builder::new()
        .metadata(separate_metadata_pda)
        .mint(separate_mint, false)
        .authority(ctx.solana_wallet)
        .payer(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .system_program(solana_sdk::system_program::id())
        .sysvar_instructions(solana_sdk::sysvar::instructions::id())
        .spl_token_program(Some(spl_token_2022::id()))
        .name("Separate Token".to_string())
        .symbol("SEP".to_string())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .decimals(9)
        .token_standard(TokenStandard::Fungible)
        .is_mutable(false)
        .instruction();

    ctx.send_solana_tx(&[create_metadata_ix]).await.unwrap();

    // Deploy a local interchain token
    let salt = solana_sdk::keccak::hash(b"NoMinterMismatchedToken").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "No Minter Token".to_owned(),
        "NMT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix])
        .await
        .expect("InterchainToken deployment failed");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    // Get the metadata account for the actual interchain token
    let (interchain_token_metadata_pda, _) = Metadata::find_pda(&interchain_token_pda);

    // Deploy remote without minter
    let deploy_remote_ix = axelar_solana_its::instruction::deploy_remote_interchain_token(
        ctx.solana_wallet,
        salt,
        "ethereum".to_string(),
        0,
    )?;

    // Get the accounts from the instruction
    let mut accounts = deploy_remote_ix.accounts.clone();

    // Replace the mint account with our separate mint
    accounts[1].pubkey = separate_mint;
    // The metadata account is at position 2
    accounts[2].pubkey = interchain_token_metadata_pda;

    // Create the modified instruction
    let mismatched_ix = Instruction {
        program_id: axelar_solana_its::id(),
        accounts,
        data: deploy_remote_ix.data,
    };

    let result = ctx.send_solana_tx(&[mismatched_ix]).await;

    // Transaction should fail
    assert!(
        result.is_err(),
        "Expected deployment to fail with mismatched metadata"
    );

    let err = result.unwrap_err();
    let error_logs = err.metadata.unwrap().log_messages;

    // Check for the specific error message
    let has_invalid_mint_error = error_logs
        .iter()
        .any(|log| log.contains("The metadata and mint accounts passed don't match"));

    assert!(
        has_invalid_mint_error,
        "Expected 'The metadata and mint accounts passed don't match' error for deployment without minter"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_interchain_token_with_mismatched_token_manager(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // First, deploy two separate local interchain tokens
    let salt1 = solana_sdk::keccak::hash(b"FirstToken").0;
    let deploy_local_ix1 = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt1,
        "First Token".to_owned(),
        "FIRST".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix1])
        .await
        .expect("First InterchainToken deployment failed");

    let salt2 = solana_sdk::keccak::hash(b"SecondToken").0;
    let deploy_local_ix2 = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt2,
        "Second Token".to_owned(),
        "SECOND".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix2])
        .await
        .expect("Second InterchainToken deployment failed");

    // Get the token IDs and mint addresses for both tokens
    let token_id2 = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt2);
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let (mint2, _) = axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id2);

    // Approve remote deployment for the first token
    let approve_remote_deployment1 =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_wallet,
            ctx.solana_wallet,
            salt1,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
        )?;

    ctx.send_solana_tx(&[approve_remote_deployment1])
        .await
        .unwrap();

    // Try to deploy remote for the first token but use the second token's mint and metadata
    let deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            ctx.solana_wallet,
            salt1,
            ctx.solana_wallet,
            "ethereum".to_string(),
            vec![1, 2, 3, 4],
            0,
        )?;

    // Get the accounts from the instruction
    let mut accounts = deploy_remote_ix.accounts.clone();

    // Replace the mint account with the second token's mint
    // but keep the token manager for the first token (salt1)
    accounts[1].pubkey = mint2;
    let (metadata2_pda, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint2);
    accounts[2].pubkey = metadata2_pda;

    // Create the modified instruction
    let mismatched_token_manager_ix = Instruction {
        program_id: axelar_solana_its::id(),
        accounts,
        data: deploy_remote_ix.data,
    };

    let result = ctx.send_solana_tx(&[mismatched_token_manager_ix]).await;

    // Transaction should fail
    assert!(
        result.is_err(),
        "Expected deployment to fail with mismatched token manager"
    );

    let err = result.unwrap_err();
    let error_logs = err.metadata.unwrap().log_messages;

    // Check for the specific error message
    let has_token_manager_error = error_logs
        .iter()
        .any(|log| log.contains("TokenManager doesn't match mint"));

    assert!(
        has_token_manager_error,
        "Expected 'TokenManager doesn't match mint' error"
    );

    Ok(())
}
