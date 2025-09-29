use axelar_solana_gateway_test_fixtures::{
    assert_msg_present_in_logs,
    gateway::{get_gateway_events, ProgramInvocationState},
};
use axelar_solana_its::state::token_manager::Type;
use axelar_solana_memo_program::get_counter_pda;
use evm_contracts_test_suite::ethers::signers::Signer as EvmSigner;
use interchain_token_transfer_gmp::GMPPayload;
use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use test_context::test_context;

use crate::{BorshPdaAccount, ItsTestContext};

/// Common test setup data
struct TestSetup {
    payer: Pubkey,
    token_id: [u8; 32],
    token_mint: Pubkey,
    token_program: Pubkey,
    counter_pda: Pubkey,
    counter_pda_ata: Pubkey,
    its_root_pda: Pubkey,
    token_manager_pda: Pubkey,
    token_manager_ata: Pubkey,
    gateway_root_pda: Pubkey,
    gas_service_root_pda: Pubkey,
}

/// Initialize common test components and PDAs
fn setup_test_environment(ctx: &ItsTestContext) -> TestSetup {
    let payer = ctx.solana_wallet;
    let token_id = ctx.deployed_interchain_token;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda();
    let token_manager_pda = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id).0;
    let token_mint = axelar_solana_its::find_interchain_token_pda(&its_root_pda, &token_id).0;

    let (counter_pda, _) = get_counter_pda();
    let token_program = spl_token_2022::id();
    let counter_pda_ata =
        get_associated_token_address_with_program_id(&counter_pda, &token_mint, &token_program);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &token_mint,
        &token_program,
    );

    TestSetup {
        payer,
        token_id,
        token_mint,
        token_program,
        counter_pda,
        counter_pda_ata,
        its_root_pda,
        token_manager_pda,
        token_manager_ata,
        gateway_root_pda: ctx.solana_chain.gateway_root_pda,
        gas_service_root_pda: ctx.solana_gas_utils.config_pda,
    }
}

/// Create ATA and mint tokens to the counter PDA
async fn setup_counter_pda_with_tokens(
    ctx: &mut ItsTestContext,
    setup: &TestSetup,
    mint_amount: u64,
) {
    // Create the counter PDA's ATA
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &setup.payer,
        &setup.counter_pda,
        &setup.token_mint,
        &setup.token_program,
    );
    ctx.send_solana_tx(&[create_ata_ix]).await.unwrap();

    // Mint tokens to the counter PDA's account
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        setup.token_id,
        setup.token_mint,
        setup.counter_pda_ata,
        setup.payer,
        setup.token_program,
        mint_amount,
    )
    .unwrap();
    ctx.send_solana_tx(&[mint_ix]).await.unwrap();
}

/// Verify that a NativeInterchainToken type token manager exists
async fn verify_token_manager_type(ctx: &mut ItsTestContext, token_manager_pda: &Pubkey) {
    let mut token_manager_account = ctx
        .solana_chain
        .try_get_account_no_checks(token_manager_pda)
        .await
        .unwrap()
        .unwrap();
    let token_manager = token_manager_account
        .deserialize::<axelar_solana_its::state::token_manager::TokenManager>(token_manager_pda)
        .unwrap();

    let Type::NativeInterchainToken = token_manager.ty else {
        panic!("Expected NativeInterchainToken type")
    };
}

/// Extract and verify the CallContract event from a transaction
fn verify_gateway_event_and_source(
    tx: &BanksTransactionResultWithMetadata,
    expected_source: &[u8; 32],
    expected_amount: u64,
) -> GMPPayload {
    let events = get_gateway_events(tx);
    let ProgramInvocationState::Succeeded(vec_events) = &events[0] else {
        panic!("Expected successful program invocation");
    };

    let call_contract_event = vec_events
        .iter()
        .find(|(_, event)| {
            matches!(
                event,
                axelar_solana_gateway::events::GatewayEvent::CallContract(_)
            )
        })
        .expect("CallContract event not found");

    let (_, axelar_solana_gateway::events::GatewayEvent::CallContract(event)) = call_contract_event
    else {
        panic!("Expected CallContract event");
    };

    let gmp_payload = GMPPayload::decode(&event.payload).unwrap();
    let GMPPayload::SendToHub(hub_message) = &gmp_payload else {
        panic!("Expected SendToHub payload");
    };

    let GMPPayload::InterchainTransfer(transfer_message) =
        GMPPayload::decode(hub_message.payload.as_ref()).unwrap()
    else {
        panic!("Expected InterchainTransfer payload in hub message");
    };

    let source_address = transfer_message.source_address.0.as_ref();
    assert_eq!(source_address, expected_source, "Source address mismatch");

    assert_eq!(
        transfer_message.amount,
        alloy_primitives::U256::from(expected_amount)
    );

    gmp_payload
}

/// Test that demonstrates the memo program can initiate interchain transfers through its PDA
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_memo_cpi_transfer(ctx: &mut ItsTestContext) {
    let setup = setup_test_environment(ctx);
    verify_token_manager_type(ctx, &setup.token_manager_pda).await;
    setup_counter_pda_with_tokens(ctx, &setup, 1000u64).await;

    let destination_chain = ctx.evm_chain_name.clone();
    let destination_address = ctx.evm_signer.wallet.address().as_bytes().to_vec();
    let transfer_amount = 100u64;
    let gas_value = 0u128;

    let send_transfer = axelar_solana_memo_program::instruction::send_interchain_transfer(
        &ctx.solana_wallet,
        &setup.counter_pda,
        &setup.its_root_pda,
        &setup.token_manager_pda,
        &setup.token_manager_ata,
        &setup.gateway_root_pda,
        &setup.gas_service_root_pda,
        &setup.token_mint,
        &setup.token_program,
        setup.token_id,
        destination_chain,
        destination_address,
        transfer_amount,
        gas_value,
    )
    .unwrap();

    let tx = ctx.send_solana_tx(&[send_transfer]).await.unwrap();

    verify_gateway_event_and_source(
        &tx,
        &axelar_solana_memo_program::ID.to_bytes(),
        transfer_amount,
    );
}

/// Test that CPI transfers fail when initiated by non-PDA accounts
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_cpi_transfer_fails_with_non_pda_account(ctx: &mut ItsTestContext) {
    let payer = ctx.solana_wallet;
    let token_id = ctx.deployed_interchain_token;

    let token_mint = axelar_solana_its::find_interchain_token_pda(
        &axelar_solana_its::find_its_root_pda().0,
        &token_id,
    )
    .0;

    let token_program = spl_token_2022::id();

    // Use the payer's token account instead of a PDA
    let payer_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
        &payer,
        &token_mint,
        &token_program,
    );

    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer,
        &payer,
        &token_mint,
        &token_program,
    );
    ctx.send_solana_tx(&[create_ata_ix]).await.unwrap();

    let mint_amount = 1000u64;
    let mint_ix = axelar_solana_its::instruction::interchain_token::mint(
        token_id,
        token_mint,
        payer_ata,
        payer,
        token_program,
        mint_amount,
    )
    .unwrap();
    ctx.send_solana_tx(&[mint_ix]).await.unwrap();

    let cpi_transfer_ix = axelar_solana_its::instruction::cpi_interchain_transfer(
        ctx.solana_wallet,
        payer,
        payer_ata,
        token_id,
        ctx.evm_chain_name.clone(),
        ctx.evm_signer.wallet.address().as_bytes().to_vec(),
        100u64,
        token_mint,
        token_program,
        0u64,
        axelar_solana_memo_program::ID,
        vec![vec![]],
    )
    .unwrap();

    let result = ctx.send_solana_tx(&[cpi_transfer_ix]).await;
    assert!(result.is_err());

    assert_msg_present_in_logs(
        result.unwrap_err(),
        "Sender account must be owned by the source program",
    );
}

/// Test that CPI transfers fail when inconsistent seeds are provided
/// This test uses the memo program's special instruction that intentionally provides wrong seeds
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_cpi_transfer_fails_with_inconsistent_seeds(ctx: &mut ItsTestContext) {
    let setup = setup_test_environment(ctx);

    // Setup counter PDA with tokens
    setup_counter_pda_with_tokens(ctx, &setup, 1000u64).await;

    // Prepare transfer parameters
    let destination_chain = ctx.evm_chain_name.clone();
    let destination_address = ctx.evm_signer.wallet.address().as_bytes().to_vec();
    let transfer_amount = 100u64;
    let gas_value = 0u128;

    // Use the special memo instruction that provides wrong seeds
    let transfer_with_wrong_seeds =
        axelar_solana_memo_program::instruction::send_interchain_transfer_with_wrong_seeds(
            &ctx.solana_wallet,
            &setup.counter_pda,
            &setup.its_root_pda,
            &setup.token_manager_pda,
            &setup.token_manager_ata,
            &setup.gateway_root_pda,
            &setup.gas_service_root_pda,
            &setup.token_mint,
            &setup.token_program,
            setup.token_id,
            destination_chain,
            destination_address,
            transfer_amount,
            gas_value,
        )
        .unwrap();

    let result = ctx.send_solana_tx(&[transfer_with_wrong_seeds]).await;
    assert!(result.is_err());

    assert_msg_present_in_logs(result.unwrap_err(), "PDA derivation mismatch");
}

/// Test that demonstrates the memo program can initiate CallContractWithInterchainToken through its PDA
/// This sends tokens along with additional data to execute on the destination contract
#[test_context(ItsTestContext)]
#[tokio::test]
async fn test_memo_cpi_call_contract_with_interchain_token(ctx: &mut ItsTestContext) {
    let setup = setup_test_environment(ctx);
    verify_token_manager_type(ctx, &setup.token_manager_pda).await;
    setup_counter_pda_with_tokens(ctx, &setup, 1000u64).await;

    let destination_chain = ctx.evm_chain_name.clone();
    let destination_address = ctx.evm_signer.wallet.address().as_bytes().to_vec();
    let transfer_amount = 100u64;
    let gas_value = 0u128;
    let custom_data = b"execute_special_function_with_params".to_vec();

    // Create the CallContractWithInterchainToken instruction through memo program
    let call_contract_transfer =
        axelar_solana_memo_program::instruction::call_contract_with_interchain_token(
            &ctx.solana_wallet,
            &setup.counter_pda,
            &setup.its_root_pda,
            &setup.token_manager_pda,
            &setup.token_manager_ata,
            &setup.gateway_root_pda,
            &setup.gas_service_root_pda,
            &setup.token_mint,
            &setup.token_program,
            setup.token_id,
            destination_chain,
            destination_address,
            transfer_amount,
            custom_data.clone(),
            gas_value,
        )
        .unwrap();

    let tx = ctx.send_solana_tx(&[call_contract_transfer]).await.unwrap();

    let gmp_payload = verify_gateway_event_and_source(
        &tx,
        &axelar_solana_memo_program::ID.to_bytes(),
        transfer_amount,
    );

    let GMPPayload::SendToHub(hub_message) = gmp_payload else {
        panic!("Expected SendToHub payload");
    };

    let GMPPayload::InterchainTransfer(transfer_message) =
        GMPPayload::decode(hub_message.payload.as_ref()).unwrap()
    else {
        panic!("Expected InterchainTransfer payload in hub message");
    };

    assert!(
        !transfer_message.data.is_empty(),
        "Transfer should include custom data"
    );

    assert_eq!(
        transfer_message.data.as_ref(),
        custom_data.as_slice(),
        "Custom data should match what was sent"
    );
}
