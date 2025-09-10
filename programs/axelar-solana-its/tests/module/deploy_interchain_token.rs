use anyhow::anyhow;
use axelar_solana_its::instruction::InterchainTokenServiceInstruction;
use borsh::to_vec;
use event_utils::Event as _;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::system_program;
use solana_program_test::tokio;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use test_context::test_context;

use axelar_solana_gateway_test_fixtures::base::FindLog;

use crate::ItsTestContext;

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_no_minter_and_no_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let salt = solana_sdk::keccak::hash(b"NoMinterNoSupplyToken").0;
    let initial_supply = 0u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "No Supply No Minter Token".to_owned(),
        "NSMT".to_owned(),
        9,
        initial_supply,
        None,
    )?;

    let result = ctx.send_solana_tx(&[deploy_local_ix]).await;

    assert!(result.is_err(), "Expected transaction to fail");
    let err = result.unwrap_err();

    let error_logs = err.metadata.unwrap().log_messages;
    let has_invalid_arg_error = error_logs
        .iter()
        .any(|log| log.contains("invalid program argument"));

    assert!(
        has_invalid_arg_error,
        "Expected InvalidArgument error when deploying with no minter and no initial supply"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_minter_but_no_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"MinterNoSupplyToken").0;
    let initial_supply = 0u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Zero Supply Token".to_owned(),
        "ZST".to_owned(),
        9,
        initial_supply,
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
        deploy_event.name, "Zero Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "ZST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_opt = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?;

    if let Some(token_account) = token_account_opt {
        let account = spl_token_2022::state::Account::unpack_from_slice(&token_account.data)?;
        assert_eq!(account.amount, 0, "Initial supply should be zero");
    }

    let mint_amount = 500u64;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        token_id,
        interchain_token_pda,
        ctx.solana_wallet,
        ctx.solana_wallet,
        spl_token_2022::id(),
        mint_amount,
    )?;

    ctx.send_solana_tx(&[mint_ix])
        .await
        .expect("Minting tokens failed");

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, mint_amount,
        "Minted amount doesn't match expected amount"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_large_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"LargeSupplyTestToken").0;
    let initial_supply = u64::MAX;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Large Supply Token".to_owned(),
        "LST".to_owned(),
        9,
        initial_supply,
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
        deploy_event.name, "Large Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "LST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, initial_supply,
        "Initial supply doesn't match expected amount"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_interchain_token_with_no_minter_but_initial_supply(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let salt = solana_sdk::keccak::hash(b"NoMinterWithSupplyToken").0;
    let initial_supply = 1000u64;

    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        salt,
        "Fixed Supply Token".to_owned(),
        "FST".to_owned(),
        9,
        initial_supply,
        None,
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
        deploy_event.name, "Fixed Supply Token",
        "token name does not match"
    );
    assert_eq!(deploy_event.symbol, "FST", "token symbol does not match");

    let token_id = axelar_solana_its::interchain_token_id(&ctx.solana_wallet, &salt);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id);

    let payer_ata = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let token_account_data = ctx
        .solana_chain
        .try_get_account_no_checks(&payer_ata)
        .await?
        .ok_or_else(|| anyhow!("token account not found"))?
        .data;

    let account = spl_token_2022::state::Account::unpack_from_slice(&token_account_data)?;

    assert_eq!(
        account.amount, initial_supply,
        "Initial supply doesn't match expected amount"
    );

    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        ctx.solana_wallet,
        token_id,
        interchain_token_pda,
        payer_ata,
        ctx.solana_wallet,
        spl_token_2022::id(),
        100,
    )?;

    let result = ctx.send_solana_tx(&[mint_ix]).await;

    assert!(
        result.is_err(),
        "Expected minting to fail for fixed supply token"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_prevent_deploy_approval_bypass(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let destination_chain = "ethereum";
    let destination_minter = vec![1, 2, 3, 4, 5];

    // Alice is our ctx.solana_chain.fixture.payer who has deployed TokenA
    let token_a_id = ctx.deployed_interchain_token;
    let token_a_salt = solana_sdk::keccak::hash(b"TestTokenSalt").0;

    // Create Bob who will deploy TokenB
    let bob = Keypair::new();

    // Fund Bob's account so he can pay for transactions
    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &bob.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    // Bob deploys TokenB
    let token_b_salt = [1u8; 32];
    let deploy_token_b_ix = axelar_solana_its::instruction::deploy_interchain_token(
        bob.pubkey(),
        token_b_salt,
        "Token B".to_string(),
        "TOKB".to_string(),
        8,
        0,
        Some(bob.pubkey()),
    )?;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[deploy_token_b_ix],
            &[
                &bob.insecure_clone(),
                &ctx.solana_chain.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let token_b_id = axelar_solana_its::interchain_token_id(&bob.pubkey(), &token_b_salt);

    // Alice creates an approval for deploying TokenA to a remote chain
    let approve_deploy_a_ix =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_chain.fixture.payer.pubkey(),
            ctx.solana_chain.fixture.payer.pubkey(),
            token_a_salt,
            destination_chain.to_string(),
            destination_minter.clone(),
        )?;

    ctx.send_solana_tx(&[approve_deploy_a_ix]).await.unwrap();

    // Find approval PDA for TokenA
    let (approval_pda, _) = axelar_solana_its::find_deployment_approval_pda(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &token_a_id,
        destination_chain,
    );

    // Verify approval account was created correctly
    let approval_account = ctx
        .solana_chain
        .try_get_account_no_checks(&approval_pda)
        .await?
        .ok_or_else(|| anyhow!("approval account not found"))?;

    assert_eq!(
        approval_account.owner,
        axelar_solana_its::id(),
        "Approval account has wrong owner"
    );

    // Now try to exploit by using TokenA's approval to deploy TokenB remotely
    // First, build the proper instruction for deploying TokenB
    let deploy_token_b_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            bob.pubkey(),
            token_b_salt,
            bob.pubkey(),
            destination_chain.to_string(),
            destination_minter.clone(),
            0, // gas value
        )?;

    let (token_b_approval_pda, _) = axelar_solana_its::find_deployment_approval_pda(
        &bob.pubkey(),
        &token_b_id,
        destination_chain,
    );

    // Get the accounts from the legitimate instruction
    let mut accounts = deploy_token_b_remote_ix.accounts.clone();

    // Find the approval account in the accounts list (usually the 4th account)
    // and replace it with Alice's approval for TokenA
    for account in accounts.iter_mut() {
        if account.pubkey == token_b_approval_pda {
            account.pubkey = approval_pda; // Replace with Alice's approval for TokenA
            break;
        }
    }

    // Create an exploitative instruction that uses TokenA's approval for TokenB deployment
    let exploit_ix = solana_program::instruction::Instruction {
        program_id: axelar_solana_its::id(),
        accounts,
        data: deploy_token_b_remote_ix.data,
    };

    let result = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[exploit_ix],
            &[
                &bob.insecure_clone(),
                &ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    // Transaction should fail due to proper validation in use_deploy_approval. Since we're
    // tempering with the accounts, sometimes the error returned is due to invalid seeds deriving
    // the PDA within `assert_valid_deploy_approval_pda`->`create_deployment_approval_pda` function
    // call.
    let err = result.as_ref().expect_err("Expected transaction to fail");
    assert!(
        err.find_log("Invalid DeploymentApproval PDA provided")
            .is_some()
            || err
                .find_log("Provided seeds do not result in a valid address")
                .is_some()
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_prevent_deploy_approval_created_by_anyone(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Bob is our ctx.solana_chain.fixture.payer who has deployed TokenB
    let token_b_salt = solana_sdk::keccak::hash(b"TestTokenSalt").0;

    // Create Alice who will deploy worthless TokenA
    let alice = Keypair::new();

    // Fund Alice's account so she can pay for transactions
    ctx.send_solana_tx(&[system_instruction::transfer(
        &ctx.solana_chain.fixture.payer.pubkey(),
        &alice.pubkey(),
        u32::MAX.into(),
    )])
    .await
    .unwrap();

    // Alice deploys TokenA
    let token_a_salt = [1u8; 32];
    let deploy_token_a_ix = axelar_solana_its::instruction::deploy_interchain_token(
        alice.pubkey(),
        token_a_salt,
        "Token A".to_string(),
        "TOKA".to_string(),
        8,
        0,
        Some(alice.pubkey()),
    )?;
    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[deploy_token_a_ix],
            &[
                &alice.insecure_clone(),
                &ctx.solana_chain.payer.insecure_clone(),
            ],
        )
        .await
        .unwrap();

    let destination_chain = "ethereum";
    let destination_minter = vec![1, 2, 3, 4, 5];

    // Alice uses here Minter role over TokenA to create the approval on TokenB
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let token_id = axelar_solana_its::interchain_token_id(&alice.pubkey(), &token_a_salt);
    let (token_manager_pda, _) =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id);
    let (roles_pda, _) = role_management::find_user_roles_pda(
        &axelar_solana_its::ID,
        &token_manager_pda,
        &alice.pubkey(),
    );
    let (deploy_approval_pda, _) = axelar_solana_its::find_deployment_approval_pda(
        &alice.pubkey(),
        &ctx.deployed_interchain_token,
        destination_chain,
    );

    let accounts = vec![
        AccountMeta::new(alice.pubkey(), true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(roles_pda, false),
        AccountMeta::new(deploy_approval_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken {
            deployer: ctx.solana_chain.fixture.payer.pubkey(),
            salt: token_b_salt,
            destination_chain: destination_chain.to_string(),
            destination_minter,
        },
    )?;

    let approve_deploy_b_ix = Instruction {
        program_id: axelar_solana_its::ID,
        accounts,
        data,
    };

    let res = ctx
        .solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[approve_deploy_b_ix],
            &[
                alice.insecure_clone(),
                ctx.solana_chain.payer.insecure_clone(),
            ],
        )
        .await;
    assert!(res.is_err());
    let err = res.as_ref().expect_err("Expected to fail");
    assert!(
        err.find_log("Invalid TokenManager PDA provided").is_some()
            || err
                .find_log("Provided seeds do not result in a valid address")
                .is_some()
    );
    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deploy_remote_interchain_token_payer_must_be_signer(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let destination_chain = "ethereum";
    let destination_minter = vec![1, 2, 3, 4, 5];
    // Use the already deployed token salt from the test context setup
    let salt = solana_sdk::keccak::hash(b"TestTokenSalt").0;
    let fake_payer = Pubkey::new_unique();

    let approve_deploy_ix = axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
        ctx.solana_chain.fixture.payer.pubkey(),
        ctx.solana_chain.fixture.payer.pubkey(),
        salt,
        destination_chain.to_string(),
        destination_minter.clone(),
    )?;

    ctx.send_solana_tx(&[approve_deploy_ix]).await.unwrap();

    let mut deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            fake_payer,
            salt,
            ctx.solana_chain.fixture.payer.pubkey(),
            destination_chain.to_string(),
            destination_minter,
            0,
        )?;

    deploy_remote_ix.accounts[0].is_signer = false;

    let result = ctx.send_solana_tx(&[deploy_remote_ix]).await;

    assert!(
        result.is_err(),
        "Expected transaction to fail when payer is not a signer"
    );

    let err = result.unwrap_err();
    let error_logs = err.metadata.unwrap().log_messages;

    let has_payer_warning = error_logs
        .iter()
        .any(|log| log.contains("Payer should be a signer"));

    assert!(
        has_payer_warning,
        "Expected warning about payer being a signer to be logged"
    );

    let has_missing_signature_error = error_logs
        .iter()
        .any(|log| log.contains("missing required signature"));

    assert!(
        has_missing_signature_error,
        "Expected MissingRequiredSignature error when payer is not a signer"
    );

    Ok(())
}
