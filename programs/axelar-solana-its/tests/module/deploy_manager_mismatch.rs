use axelar_solana_gateway_test_fixtures::base::FindLog;
use solana_program_test::tokio;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer as _;
use test_context::test_context;

use crate::ItsTestContext;

async fn deploy_interchain_token_for_user(
    ctx: &mut ItsTestContext,
    user: &Keypair,
    salt: [u8; 32],
    name: &str,
    symbol: &str,
) -> anyhow::Result<[u8; 32]> {
    let deploy_token_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_chain.fixture.payer.pubkey(),
        user.pubkey(),
        salt,
        name.to_owned(),
        symbol.to_owned(),
        9,
        1000,
        Some(user.pubkey()),
    )?;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[deploy_token_ix],
            &[
                user.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    Ok(axelar_solana_its::interchain_token_id(
        &user.pubkey(),
        &salt,
    ))
}

async fn create_deployment_approval(
    ctx: &mut ItsTestContext,
    minter: &Keypair,
    salt: [u8; 32],
    destination_chain: &str,
    destination_minter: &[u8],
) -> anyhow::Result<()> {
    let approve_deployment_ix =
        axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
            ctx.solana_chain.fixture.payer.pubkey(),
            minter.pubkey(),
            minter.pubkey(), // minter is the deployer
            salt,
            destination_chain.to_string(),
            destination_minter.to_vec(),
        )?;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[approve_deployment_ix],
            &[
                minter.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    Ok(())
}

async fn attempt_deployment_with_specific_token_manager(
    ctx: &mut ItsTestContext,
    deployer: &Keypair,
    target_token_salt: [u8; 32],
    manager_token_id: [u8; 32],
    destination_chain: &str,
    destination_minter: &[u8],
) -> Result<
    solana_program_test::BanksTransactionResultWithMetadata,
    solana_program_test::BanksTransactionResultWithMetadata,
> {
    let mut deploy_remote_ix =
        axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
            ctx.solana_chain.fixture.payer.pubkey(),
            deployer.pubkey(),
            target_token_salt,
            deployer.pubkey(),
            destination_chain.to_string(),
            destination_minter.to_vec(),
            0,
        )
        .expect("Failed to create deploy instruction");

    // Replace target token's token_manager_pda with fake token's token_manager_pda,
    // trying to use their current minter privileges on fake token to deploy target token
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let fake_token_manager_pda =
        axelar_solana_its::find_token_manager_pda(&its_root_pda, &manager_token_id).0;
    deploy_remote_ix.accounts[5].pubkey = fake_token_manager_pda;

    ctx.solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[deploy_remote_ix],
            &[
                deployer.insecure_clone(),
                ctx.solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_deployment_with_token_manager_mismatch(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    let alice = Keypair::new();
    ctx.solana_chain
        .fixture
        .fund_account(&alice.pubkey(), 10_000_000_000)
        .await;

    let salt_a = solana_sdk::keccak::hash(b"token-a-salt").to_bytes();
    let salt_b = solana_sdk::keccak::hash(b"token-b-salt").to_bytes();
    let destination_chain = "ethereum";
    let destination_minter = b"0x1234567890123456789012345678901234567890";

    let token_id_a = deploy_interchain_token_for_user(ctx, &alice, salt_a, "Token A", "TA").await?;

    create_deployment_approval(ctx, &alice, salt_a, destination_chain, destination_minter).await?;

    // Alice creates TokenB and becomes its minter
    let token_id_b = deploy_interchain_token_for_user(ctx, &alice, salt_b, "Token B", "TB").await?;

    // Attempt to deploy TokenA remotely using TokenB's token manager for authorization
    // This should fail.
    {
        let result = attempt_deployment_with_specific_token_manager(
            ctx,
            &alice,
            salt_a,     // Target token (TokenA) salt
            token_id_b, // Fake token (TokenB) ID for authorization
            destination_chain,
            destination_minter,
        )
        .await;

        assert!(
            result.is_err(),
            "Expected transaction to fail due to token manager/mint mismatch"
        );

        let error_tx = result.unwrap_err();
        assert!(
            error_tx
                .find_log("Derived PDA doesn't match given roles account address")
                .is_some(),
            "Expected roles validation error message"
        );
    }

    // Attempt to deploy TokenB remotely using TokenA's token manager for authorization
    // This should fail as well.
    {
        let result = attempt_deployment_with_specific_token_manager(
            ctx,
            &alice,
            salt_b,
            token_id_a,
            destination_chain,
            destination_minter,
        )
        .await;

        assert!(
            result.is_err(),
            "Expected transaction to fail due to token manager/mint mismatch"
        );

        let error_tx = result.unwrap_err();
        assert!(
            error_tx
                .find_log("Derived PDA doesn't match given roles account address")
                .is_some(),
            "Expected roles validation error message"
        );
    }

    // Attempt to deploy TokenB
    // This should fail because no approval was given for TokenB
    {
        let result = attempt_deployment_with_specific_token_manager(
            ctx,
            &alice,
            salt_b,
            token_id_b,
            destination_chain,
            destination_minter,
        )
        .await;

        assert!(
            result.is_err(),
            "Expected transaction to fail due to token manager/mint mismatch"
        );

        let error_tx = result.unwrap_err();
        assert!(
            error_tx.find_log("Warning: failed to deserialize account as axelar_solana_its::state::deploy_approval::DeployApproval: Unexpected length of input. The account might not have been initialized.").is_some(),
            "Expected deserialization error message because the account doesn't exist (because no approval was created for TokenB)"
        );
    }

    // Now do that with the correct token manager (TokenA's) for TokenA deployment.
    // This should succeed because approval was given for TokenA.
    attempt_deployment_with_specific_token_manager(
        ctx,
        &alice,
        salt_a,
        token_id_a,
        destination_chain,
        destination_minter,
    )
    .await
    .unwrap();

    Ok(())
}
