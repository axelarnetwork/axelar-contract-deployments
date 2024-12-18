#![cfg(test)]
#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]

use alloy_primitives::Bytes;
use axelar_solana_gateway::get_incoming_message_pda;
use axelar_solana_gateway::state::incoming_message::{command_id, MessageStatus};
use axelar_solana_its::instructions::{DeployInterchainTokenInputs, InterchainTransferInputs};
use axelar_solana_its::state::token_manager;
use axelar_solana_its::state::token_manager::TokenManager;
use evm_contracts_test_suite::ethers::signers::Signer as EvmSigner;
use evm_contracts_test_suite::ethers::types::U256;
use interchain_token_transfer_gmp::InterchainTransfer;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program_test::tokio;
use solana_sdk::clock::Clock;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_associated_token_account::instruction::create_associated_token_account;

use crate::{
    axelar_evm_setup, axelar_solana_setup, call_evm, call_solana_gateway,
    ensure_evm_gateway_approval, prepare_evm_approve_contract_call, prepare_receive_from_hub,
    program_test, random_hub_message_with_destination_and_payload, retrieve_evm_log_with_filter,
    route_its_hub, ItsProgramWrapper, ITS_HUB_TRUSTED_CHAIN_NAME,
};

// Test that the flow limit is enforced for incoming interchain transfers.
// The limit is set to 800, we test that a transfer with 500 tokens is
// successful and a transfer with 1000 tokens fails.
#[rstest::rstest]
#[case(1000_u64)]
#[should_panic]
#[case(500_u64)]
#[tokio::test]
async fn test_incoming_interchain_transfer_with_limit(#[case] flow_limit: u64) {
    use axelar_solana_its::instructions::ItsGmpInstructionInputs;
    use borsh::BorshDeserialize;

    let mut solana_chain = program_test().await;
    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let token_program_id = spl_token_2022::id();

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    let token_id = Pubkey::create_with_seed(&its_root_pda, "test_token", &axelar_solana_its::id())
        .unwrap()
        .to_bytes();
    let token_manager_pda = axelar_solana_its::find_token_manager_pda(&its_root_pda, &token_id).0;
    let mint = solana_chain
        .fixture
        .init_new_mint(solana_chain.fixture.payer.pubkey(), token_program_id, 18)
        .await;

    let inner_payload = GMPPayload::DeployTokenManager(DeployTokenManager {
        selector: alloy_primitives::Uint::<256, 4>::from(2_u128),
        token_id: token_id.into(),
        token_manager_type: token_manager::Type::LockUnlock.into(),
        params: axelar_solana_its::state::token_manager::encode_params(
            None,
            Some(solana_chain.fixture.payer.pubkey()),
            mint,
        )
        .into(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_payload, "ethereum".to_owned());
    let payload_hash = solana_sdk::keccak::hash(&its_gmp_payload.encode()).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .token_program(spl_token_2022::id())
        .build();

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    solana_chain.fixture.send_tx(&[instruction]).await.unwrap();

    // Assert
    // Message should be executed
    let gateway_approved_message = solana_chain.incoming_message(incoming_message_pda).await;
    assert_eq!(gateway_approved_message.status, MessageStatus::Executed);

    let data = solana_chain
        .fixture
        .get_account(&token_manager_pda, &axelar_solana_its::id())
        .await
        .data;
    let token_manager = TokenManager::try_from_slice(&data).unwrap();

    assert_eq!(token_manager.token_id.as_ref(), token_id.as_ref());
    assert_eq!(mint.as_ref(), token_manager.token_address.as_ref());

    let flow_limit_ix = axelar_solana_its::instructions::set_flow_limit(
        solana_chain.fixture.payer.pubkey(),
        token_id,
        flow_limit,
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[flow_limit_ix]).await;

    let token_manager_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &token_manager_pda,
            &mint,
            &token_program_id,
        );

    // Fund the token manager to simulate locked tokens.
    let locked_amount = 5000_u64;
    solana_chain
        .fixture
        .mint_tokens_to(
            &mint,
            &token_manager_ata,
            &solana_chain.fixture.payer.insecure_clone(),
            locked_amount,
            &token_program_id,
        )
        .await;

    let transferred_amount = 800_u64;
    let inner_transfer_payload = GMPPayload::InterchainTransfer(InterchainTransfer {
        selector: alloy_primitives::Uint::<256, 4>::from(0_u128),
        token_id: token_id.into(),
        source_address: token_id.into(), // Does't matter
        destination_address: solana_chain.fixture.payer.pubkey().to_bytes().into(),
        amount: alloy_primitives::Uint::<256, 4>::from(transferred_amount),
        data: Bytes::new(),
    });

    let its_gmp_payload = prepare_receive_from_hub(&inner_transfer_payload, "ethereum".to_owned());
    let payload_hash = solana_sdk::keccak::hash(&its_gmp_payload.encode()).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) =
        get_incoming_message_pda(&command_id(&message.cc_id.chain, &message.cc_id.id));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let clock_sysvar = solana_chain
        .fixture
        .banks_client
        .get_sysvar::<Clock>()
        .await
        .unwrap();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message(merkelised_message.leaf.message)
        .payload(its_gmp_payload)
        .timestamp(clock_sysvar.unix_timestamp)
        .token_program(token_program_id)
        .mint(mint)
        .build();

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    solana_chain.fixture.send_tx(&[instruction]).await.unwrap();

    // Assert
    // Message should be executed
    let gateway_approved_message = solana_chain.incoming_message(incoming_message_pda).await;
    assert_eq!(gateway_approved_message.status, MessageStatus::Executed);

    let token_manager_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(token_manager_ata)
        .await
        .unwrap();

    let destination_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            &solana_chain.fixture.payer.pubkey(),
            &mint,
            &token_program_id,
        );

    let destination_ata_account = solana_chain
        .fixture
        .banks_client
        .get_packed_account_data::<spl_token_2022::state::Account>(destination_ata)
        .await
        .unwrap();

    assert_eq!(
        token_manager_ata_account.amount,
        locked_amount - transferred_amount,
        "New balance doesn't match expected balance"
    );

    assert_eq!(
        destination_ata_account.amount, transferred_amount,
        "New balance doesn't match expected balance"
    );
}

// Test that the flow limit is enforced for outgoing interchain transfers.
// We try to transfer 800 tokens with two different limits: 1000 and 500. The latter should fail.
#[rstest::rstest]
#[case(1000_u64)]
#[should_panic]
#[case(500_u64)]
#[tokio::test]
async fn test_outgoing_interchain_transfer_with_limit(#[case] flow_limit: u64) {
    use axelar_solana_gateway::processor::GatewayEvent;
    use axelar_solana_gateway_test_fixtures::gateway::ProgramInvocationState;

    let ItsProgramWrapper {
        mut solana_chain,
        chain_name: solana_id,
        ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;
    let gas_utils = solana_chain.fixture.deploy_gas_service().await;
    solana_chain
        .fixture
        .init_gas_config(&gas_utils)
        .await
        .unwrap();

    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy_local = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .salt(salt)
        .minter(solana_chain.fixture.payer.pubkey().as_ref().to_vec())
        .gas_value(0)
        .gas_service(axelar_solana_gas_service::id())
        .gas_config_pda(gas_utils.config_pda)
        .build();

    let deploy_local_ix =
        axelar_solana_its::instructions::deploy_interchain_token(deploy_local).unwrap();
    solana_chain
        .fixture
        .send_tx(&[deploy_local_ix])
        .await
        .unwrap();

    let deploy_remote = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .destination_chain(destination_chain.clone())
        .salt(salt)
        .minter(evm_signer.wallet.address().as_bytes().to_vec())
        .gas_value(0)
        .gas_service(axelar_solana_gas_service::id())
        .gas_config_pda(gas_utils.config_pda)
        .build();

    let deploy_remote_ix =
        axelar_solana_its::instructions::deploy_interchain_token(deploy_remote.clone()).unwrap();

    let emitted_events = call_solana_gateway(&mut solana_chain.fixture, deploy_remote_ix)
        .await
        .pop()
        .unwrap();
    dbg!("Deployed remote");

    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };

    let [(_, GatewayEvent::CallContract(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };

    let payload = route_its_hub(
        GMPPayload::decode(&emitted_event.payload).unwrap(),
        solana_id.clone(),
    );
    let encoded_payload = payload.encode();

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_sdk::keccak::hash(&encoded_payload).0,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );

    let mut message = messages[0].clone();
    ITS_HUB_TRUSTED_CHAIN_NAME.clone_into(&mut message.source_chain);

    let command_id =
        ensure_evm_gateway_approval(message.clone(), proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        encoded_payload.into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_token_deployed_filter(),
    )
    .await;

    let expected_token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        solana_sdk::keccak::hash(b"our cool interchain token")
            .0
            .as_slice(),
    );

    assert_eq!(log.token_id, expected_token_id, "token_id does not match");

    let flow_limit_ix = axelar_solana_its::instructions::set_flow_limit(
        solana_chain.fixture.payer.pubkey(),
        expected_token_id,
        flow_limit,
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[flow_limit_ix]).await;

    let (its_root_pda, _) = axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    let (interchain_token_pda, _) =
        axelar_solana_its::find_interchain_token_pda(&its_root_pda, &log.token_id);

    let associated_account_address = get_associated_token_address_with_program_id(
        &solana_chain.fixture.payer.pubkey(),
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    let create_token_account_ix = create_associated_token_account(
        &solana_chain.fixture.payer.pubkey(),
        &solana_chain.fixture.payer.pubkey(),
        &interchain_token_pda,
        &spl_token_2022::id(),
    );

    solana_chain
        .fixture
        .send_tx(&[create_token_account_ix])
        .await;

    let mint_ix = axelar_solana_its::instructions::interchain_token::mint(
        expected_token_id,
        interchain_token_pda,
        associated_account_address,
        solana_chain.fixture.payer.pubkey(),
        spl_token_2022::id(),
        900,
    )
    .unwrap();

    solana_chain.fixture.send_tx(&[mint_ix]).await;

    let clock_sysvar: Clock = solana_chain
        .fixture
        .banks_client
        .get_sysvar()
        .await
        .unwrap();

    dbg!("Will transfer");

    let transfer = InterchainTransferInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .authority(solana_chain.fixture.payer.pubkey())
        .source_account(associated_account_address)
        .token_id(log.token_id)
        .destination_chain(destination_chain)
        .destination_address(evm_signer.wallet.address().as_bytes().to_vec())
        .amount(800)
        .gas_value(0)
        .gas_service(axelar_solana_gas_service::id())
        .gas_config_pda(gas_utils.config_pda)
        .timestamp(clock_sysvar.unix_timestamp)
        .data(vec![])
        .build();

    let transfer_ix =
        axelar_solana_its::instructions::interchain_transfer(transfer.clone()).unwrap();
    let emitted_events = call_solana_gateway(&mut solana_chain.fixture, transfer_ix)
        .await
        .pop()
        .unwrap();

    let ProgramInvocationState::Succeeded(vec_events) = emitted_events else {
        panic!("unexpected event")
    };

    let [(_, GatewayEvent::CallContract(emitted_event))] = vec_events.as_slice() else {
        panic!("unexpected event")
    };

    let payload = route_its_hub(
        GMPPayload::decode(&emitted_event.payload).unwrap(),
        solana_id,
    );
    let encoded_payload = payload.encode();

    let (messages, proof) = prepare_evm_approve_contract_call(
        solana_sdk::keccak::hash(&encoded_payload).0,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );

    let mut message = messages[0].clone();
    ITS_HUB_TRUSTED_CHAIN_NAME.clone_into(&mut message.source_chain);

    let command_id =
        ensure_evm_gateway_approval(message.clone(), proof, &its_contracts.gateway).await;

    call_evm(its_contracts.interchain_token_service.execute(
        command_id,
        message.source_chain,
        message.source_address,
        encoded_payload.into(),
    ))
    .await;

    let log = retrieve_evm_log_with_filter(
        its_contracts
            .interchain_token_service
            .interchain_transfer_received_filter(),
    )
    .await;

    assert_eq!(log.amount, U256::from(800_u32));
}
