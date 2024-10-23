use axelar_solana_its::instructions::DeployInterchainTokenInputs;
use evm_contracts_test_suite::ethers::signers::Signer;
use evm_contracts_test_suite::ethers::types::Address;
use evm_contracts_test_suite::ethers::utils::keccak256;
use evm_contracts_test_suite::evm_contracts_rs::contracts::interchain_token_service::InterchainTokenDeployedFilter;
use gateway::events::{ArchivedCallContract, ArchivedGatewayEvent, EventContainer, GatewayEvent};
use solana_program_test::tokio;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer as SolanaSigner;
use solana_sdk::transaction::Transaction;

use crate::{axelar_evm_setup, axelar_solana_setup, ItsProgramWrapper};

#[tokio::test]
#[allow(clippy::panic)]
async fn test_send_from_solana_to_evm() {
    let ItsProgramWrapper {
        mut solana_chain, ..
    } = axelar_solana_setup(false).await;
    let (_evm_chain, evm_signer, its_contracts, mut weighted_signers, domain_separator) =
        axelar_evm_setup().await;

    let solana_id = "solana-localnet";
    let destination_chain = "ethereum".to_string();
    let salt = solana_sdk::keccak::hash(b"our cool interchain token").0;
    let deploy = DeployInterchainTokenInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .name("Test Token".to_owned())
        .symbol("TT".to_owned())
        .decimals(18)
        .salt(salt)
        .minter(evm_signer.wallet.address().as_bytes().to_owned())
        .destination_chain(destination_chain)
        .gas_value(0)
        .build();

    let gateway_event = call_solana_gateway(&mut solana_chain.fixture, deploy).await;
    let ArchivedGatewayEvent::CallContract(call_contract) = gateway_event.parse() else {
        panic!("Expected CallContract event, got {gateway_event:?}");
    };

    let (messages, proof) = evm_prepare_approve_contract_call(
        solana_id,
        call_contract,
        its_contracts.interchain_token_service.address(),
        &mut weighted_signers,
        domain_separator,
    );
    let message = messages[0].clone();

    its_contracts
        .interchain_token_service
        .set_trusted_address(message.source_chain.clone(), message.source_address.clone())
        .send()
        .await
        .unwrap()
        .await
        .unwrap()
        .unwrap();

    its_contracts
        .gateway
        .approve_messages(messages, proof)
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    let is_approved = its_contracts
        .gateway
        .is_message_approved(
            message.source_chain.clone(),
            message.message_id.clone(),
            message.source_address.clone(),
            message.contract_address,
            message.payload_hash,
        )
        .await
        .unwrap();

    assert!(is_approved, "contract call was not approved");
    assert_eq!(
        keccak256(&call_contract.payload),
        call_contract.payload_hash
    );

    let command_id = its_contracts
        .gateway
        .message_to_command_id(message.source_chain.clone(), message.message_id.clone())
        .await
        .unwrap();

    its_contracts
        .interchain_token_service
        .execute(
            command_id,
            message.source_chain,
            message.source_address,
            call_contract.payload.to_vec().into(),
        )
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    let logs: Vec<InterchainTokenDeployedFilter> = its_contracts
        .interchain_token_service
        .interchain_token_deployed_filter()
        .from_block(0_u64)
        .query()
        .await
        .unwrap();

    let log = logs.into_iter().next().expect("no logs found");
    let expected_token_id = axelar_solana_its::interchain_token_id(
        &solana_chain.fixture.payer.pubkey(),
        solana_sdk::keccak::hash(b"our cool interchain token")
            .0
            .as_slice(),
    );

    assert_eq!(log.token_id, expected_token_id, "token_id does not match");
}

fn evm_prepare_approve_contract_call(
    solana_id: &str,
    call_contract: &ArchivedCallContract,
    destination_address: Address,
    signer_set: &mut evm_contracts_test_suite::evm_weighted_signers::WeightedSigners,
    domain_separator: [u8; 32],
) -> (
    Vec<evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::Message>,
    evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::Proof,
) {
    let message =
        evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::Message {
            source_chain: solana_id.to_string(),
            message_id: "message555".to_string(),
            source_address: Pubkey::from(call_contract.sender).to_string(),
            // TODO: use address from the contract call once we have the trusted addresses in place
            // (the address is currently empty)
            contract_address: destination_address,
            payload_hash: call_contract.payload_hash,
        };
    let approve_contract_call_command =
        evm_contracts_test_suite::evm_weighted_signers::get_approve_contract_call(message.clone());
    // build command batch
    let signed_weighted_execute_input =
        evm_contracts_test_suite::evm_weighted_signers::get_weighted_signatures_proof(
            &approve_contract_call_command,
            signer_set,
            domain_separator,
        );
    (vec![message], signed_weighted_execute_input)
}

async fn call_solana_gateway(
    solana_fixture: &mut test_fixtures::test_setup::TestFixture,
    deploy_interchain_token: DeployInterchainTokenInputs,
) -> EventContainer {
    let transaction = Transaction::new_signed_with_payer(
        &[axelar_solana_its::instructions::deploy_interchain_token(
            &solana_fixture.payer.pubkey(),
            deploy_interchain_token,
        )
        .unwrap()],
        Some(&solana_fixture.payer.pubkey()),
        &[&solana_fixture.payer],
        solana_fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap(),
    );
    let tx = solana_fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");

    let log_msgs = tx.metadata.unwrap().log_messages;
    let gateway_event = log_msgs
        .iter()
        .find_map(GatewayEvent::parse_log)
        .expect("Gateway event was not emitted?");

    gateway_event
}
