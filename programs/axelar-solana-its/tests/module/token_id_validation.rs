use axelar_solana_gateway_test_fixtures::{assert_msg_present_in_logs, base::FindLog};
use evm_contracts_test_suite::ethers::signers::Signer;
use mpl_token_metadata::accounts::Metadata;
use mpl_token_metadata::instructions::CreateV1Builder;
use mpl_token_metadata::types::TokenStandard;
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;
use test_context::test_context;

use axelar_solana_its::state::token_manager::Type as TokenManagerType;
use evm_contracts_test_suite::evm_contracts_rs::contracts::custom_test_token::CustomTestToken;
use evm_contracts_test_suite::ContractMiddleware;
use interchain_token_transfer_gmp::GMPPayload;

use event_cpi_test_utils::get_first_event_cpi_occurrence;

use crate::ItsTestContext;

/// Helper function to set up a custom token with a specific token manager type
async fn setup_custom_token(
    ctx: &mut ItsTestContext,
    token_manager_type: TokenManagerType,
    token_name: &str,
    token_symbol: &str,
    salt_seed: &[u8],
) -> anyhow::Result<([u8; 32], CustomTestToken<ContractMiddleware>, Pubkey)> {
    let salt = solana_sdk::keccak::hash(salt_seed).to_bytes();
    let custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token(token_name.to_owned(), token_symbol.to_owned(), 18)
        .await?;

    let custom_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = Metadata::find_pda(&custom_solana_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(custom_solana_token, false)
        .authority(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .payer(ctx.solana_wallet)
        .is_mutable(false)
        .name(token_name.to_owned())
        .symbol(token_symbol.to_owned())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    let register_metadata = axelar_solana_its::instruction::register_token_metadata(
        ctx.solana_wallet,
        custom_solana_token,
        0,
    )?;

    // Simulate first to get the event
    let simulation_result = ctx
        .simulate_solana_tx(&[metadata_ix.clone(), register_metadata.clone()])
        .await;
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()[1]
        .clone();
    let call_contract_event = get_first_event_cpi_occurrence::<
        axelar_solana_gateway::events::CallContractEvent,
    >(&inner_ixs)
    .expect("CallContractEvent not found");

    // Then execute the transaction
    ctx.send_solana_tx(&[metadata_ix, register_metadata])
        .await
        .unwrap();

    let GMPPayload::RegisterTokenMetadata(register_message) =
        GMPPayload::decode(&call_contract_event.payload)?
    else {
        panic!("wrong message");
    };

    assert_eq!(
        register_message.token_address.as_ref(),
        custom_solana_token.as_ref()
    );

    ctx.evm_its_contracts
        .interchain_token_service
        .register_token_metadata(custom_token.address(), 0.into())
        .send()
        .await?
        .await?;

    let token_id = axelar_solana_its::linked_token_id(&ctx.solana_wallet, &salt);
    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        custom_solana_token,
        token_manager_type,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    let link_token_ix = axelar_solana_its::instruction::link_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        ctx.evm_chain_name.clone(),
        custom_token.address().as_bytes().to_vec(),
        token_manager_type,
        vec![],
        0,
    )?;

    // Simulate first to get the event
    let simulation_result = ctx.simulate_solana_tx(&[link_token_ix.clone()]).await;
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    let call_contract_event = get_first_event_cpi_occurrence::<
        axelar_solana_gateway::events::CallContractEvent,
    >(&inner_ixs)
    .expect("CallContractEvent not found");

    // Then execute the transaction
    ctx.send_solana_tx(&[link_token_ix]).await.unwrap();

    ctx.relay_to_evm(&call_contract_event.payload).await;

    Ok((token_id, custom_token, custom_solana_token))
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_valid_token_id_mint_matches_token_address(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Set up a custom token with MintBurn type
    let (token_id, evm_token, solana_token) = setup_custom_token(
        ctx,
        TokenManagerType::MintBurn,
        "Valid Token",
        "VT",
        b"valid-token-salt",
    )
    .await?;

    // Transfer mint authority to ITS so we can mint through ITS
    let authority_transfer_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            ctx.solana_wallet,
            token_id,
            solana_token,
            spl_token_2022::id(),
        )?;

    let token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let create_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token,
        &spl_token_2022::id(),
    );

    let initial_balance = 300;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        solana_token,
        token_account,
        ctx.solana_wallet,
        spl_token_2022::id(),
        initial_balance,
    )?;

    ctx.send_solana_tx(&[authority_transfer_ix, create_ata_ix, mint_ix])
        .await
        .unwrap();

    // Transfer mintership on EVM side
    let token_manager = ctx
        .evm_its_contracts
        .interchain_token_service
        .token_manager_address(token_id)
        .call()
        .await?;

    evm_token
        .transfer_mintership(token_manager)
        .send()
        .await?
        .await?;

    // This should succeed because the token_id corresponds to the correct mint
    let transfer_amount = 100;
    let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_account,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        solana_token,
        spl_token_2022::id(),
        0,
    )?;

    // Simulate first to get the event
    let simulation_result = ctx.simulate_solana_tx(&[transfer_ix.clone()]).await;
    let inner_ixs = simulation_result
        .simulation_details
        .unwrap()
        .inner_instructions
        .unwrap()
        .first()
        .cloned()
        .unwrap();
    let call_contract_event = get_first_event_cpi_occurrence::<
        axelar_solana_gateway::events::CallContractEvent,
    >(&inner_ixs)
    .expect("CallContractEvent not found");

    // This should succeed without errors
    ctx.send_solana_tx(&[transfer_ix]).await.unwrap();

    // Verify the transfer event was emitted successfully
    let GMPPayload::SendToHub(hub_message) = GMPPayload::decode(&call_contract_event.payload)?
    else {
        panic!("Expected SendToHub message");
    };

    let GMPPayload::InterchainTransfer(transfer_payload) =
        GMPPayload::decode(&hub_message.payload)?
    else {
        panic!("Expected InterchainTransfer message");
    };

    assert_eq!(transfer_payload.token_id, token_id);
    assert_eq!(transfer_payload.amount, transfer_amount.try_into().unwrap());

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_invalid_token_id_mint_mismatch_rejected(
    ctx: &mut ItsTestContext,
) -> anyhow::Result<()> {
    // Set up two different custom tokens
    let (token_id_a, _, solana_token_a) = setup_custom_token(
        ctx,
        TokenManagerType::MintBurn,
        "Token A",
        "TA",
        b"token-a-salt",
    )
    .await?;

    let (token_id_b, _, solana_token_b) = setup_custom_token(
        ctx,
        TokenManagerType::MintBurn,
        "Token B",
        "TB",
        b"token-b-salt",
    )
    .await?;

    // Transfer mint authority for both tokens to ITS
    let authority_transfer_a_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            ctx.solana_wallet,
            token_id_a,
            solana_token_a,
            spl_token_2022::id(),
        )?;

    let authority_transfer_b_ix =
        axelar_solana_its::instruction::token_manager::handover_mint_authority(
            ctx.solana_wallet,
            ctx.solana_wallet,
            token_id_b,
            solana_token_b,
            spl_token_2022::id(),
        )?;

    ctx.send_solana_tx(&[authority_transfer_a_ix, authority_transfer_b_ix])
        .await
        .unwrap();

    // Create token accounts for both tokens
    let token_account_a = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &solana_token_a,
        &spl_token_2022::id(),
    );

    let create_ata_a_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token_a,
        &spl_token_2022::id(),
    );

    let create_ata_b_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &solana_token_b,
        &spl_token_2022::id(),
    );

    // Mint some tokens to account A
    let initial_balance = 300;
    let mint_to_a_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id_a,
        solana_token_a,
        token_account_a,
        ctx.solana_wallet,
        spl_token_2022::id(),
        initial_balance,
    )?;

    ctx.send_solana_tx(&[create_ata_a_ix, create_ata_b_ix, mint_to_a_ix])
        .await
        .unwrap();

    // Now try to transfer using token_id_a but with mint B (this should fail after the fix)
    let transfer_amount = 100;
    let malicious_transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        token_account_a,
        token_id_b, // But using token_id_b (mismatch!)
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        solana_token_a, // With mint A (which doesn't match token_id_b's token_manager.token_address)
        spl_token_2022::id(),
        0,
    )?;

    // This should fail with "Mint and token ID don't match" error
    let result = ctx.send_solana_tx(&[malicious_transfer_ix]).await;

    assert!(
        result.is_err(),
        "Expected transaction to fail due to mint/token_id mismatch"
    );

    let error_tx = result.unwrap_err();
    assert!(
        error_tx.find_log("Mint and token ID don't match").is_some(),
        "Expected 'Mint and token ID don't match' error message"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_lock_unlock_token_id_validation(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    // Set up a LockUnlock token type for this test
    let (token_id, _, _solana_token) = setup_custom_token(
        ctx,
        TokenManagerType::LockUnlock,
        "Lock Unlock Token",
        "LUT",
        b"lock-unlock-salt",
    )
    .await?;

    // Create a different token mint (worthless token)
    let worthless_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let worthless_token_account = get_associated_token_address_with_program_id(
        &ctx.solana_wallet,
        &worthless_token,
        &spl_token_2022::id(),
    );

    let create_worthless_ata_ix = create_associated_token_account(
        &ctx.solana_wallet,
        &ctx.solana_wallet,
        &worthless_token,
        &spl_token_2022::id(),
    );

    // Mint tokens to the worthless token account (simulating worthless tokens)
    let worthless_balance = 1000;
    let mint_worthless_ix = spl_token_2022::instruction::mint_to(
        &spl_token_2022::id(),
        &worthless_token,
        &worthless_token_account,
        &ctx.solana_wallet,
        &[],
        worthless_balance,
    )?;

    ctx.send_solana_tx(&[create_worthless_ata_ix, mint_worthless_ix])
        .await
        .unwrap();

    // Try to transfer the worthless tokens using the legitimate token_id
    // This should fail because the mint doesn't match the token_manager's token_address
    let transfer_amount = 100;
    let malicious_transfer_ix = axelar_solana_its::instruction::interchain_transfer(
        ctx.solana_wallet,
        ctx.solana_wallet,
        worthless_token_account,
        token_id, // But legitimate token_id
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        transfer_amount,
        worthless_token, // Worthless mint (mismatch!)
        spl_token_2022::id(),
        0,
    )?;

    // This should fail with the validation error
    let result = ctx.send_solana_tx(&[malicious_transfer_ix]).await;

    assert!(
        result.is_err(),
        "Expected transaction to fail due to worthless token attack"
    );

    let error_tx = result.unwrap_err();
    assert!(
        error_tx.find_log("Mint and token ID don't match").is_some(),
        "Expected 'Mint and token ID don't match' error message for worthless token attack"
    );

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_self_remote_deployment_rejected(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    let salt = solana_sdk::keccak::hash(b"SelfDeployTest").0;
    let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        "Self Deploy Test Token".to_owned(),
        "SDT".to_owned(),
        9,
        1000,
        Some(ctx.solana_wallet),
    )?;

    ctx.send_solana_tx(&[deploy_local_ix]).await.unwrap();

    let deploy_remote_ix = axelar_solana_its::instruction::deploy_remote_interchain_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        ctx.solana_chain_name.clone(),
        0,
    )?;

    let result = ctx.send_solana_tx(&[deploy_remote_ix]).await;

    assert!(result.is_err());

    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "Cannot deploy remotely to the origin chain");

    Ok(())
}

#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_self_token_linking_rejected(ctx: &mut ItsTestContext) -> anyhow::Result<()> {
    use axelar_solana_its::state::token_manager::Type as TokenManagerType;

    let salt = [42u8; 32];
    let custom_token = ctx
        .evm_signer
        .deploy_axelar_custom_test_token("Link Test Token".to_owned(), "LTT".to_owned(), 18)
        .await?;

    let custom_solana_token = ctx
        .solana_chain
        .fixture
        .init_new_mint(ctx.solana_wallet, spl_token_2022::id(), 9)
        .await;

    let (metadata_pda, _) = Metadata::find_pda(&custom_solana_token);
    let metadata_ix = CreateV1Builder::new()
        .metadata(metadata_pda)
        .token_standard(TokenStandard::Fungible)
        .mint(custom_solana_token, false)
        .authority(ctx.solana_wallet)
        .update_authority(ctx.solana_wallet, true)
        .payer(ctx.solana_wallet)
        .is_mutable(false)
        .name("Link Test Token".to_owned())
        .symbol("LTT".to_owned())
        .uri(String::new())
        .seller_fee_basis_points(0)
        .instruction();

    ctx.send_solana_tx(&[metadata_ix]).await.unwrap();

    let register_custom_token_ix = axelar_solana_its::instruction::register_custom_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        custom_solana_token,
        TokenManagerType::LockUnlock,
        spl_token_2022::id(),
        None,
    )?;

    ctx.send_solana_tx(&[register_custom_token_ix])
        .await
        .unwrap();

    let link_token_ix = axelar_solana_its::instruction::link_token(
        ctx.solana_wallet,
        ctx.solana_wallet,
        salt,
        ctx.solana_chain_name.clone(),
        custom_token.address().as_bytes().to_vec(),
        TokenManagerType::LockUnlock,
        vec![],
        0,
    )?;

    let result = ctx.send_solana_tx(&[link_token_ix]).await;
    assert!(result.is_err(),);

    let error_tx = result.unwrap_err();
    assert_msg_present_in_logs(error_tx, "Cannot link to another token on the same chain");

    Ok(())
}
