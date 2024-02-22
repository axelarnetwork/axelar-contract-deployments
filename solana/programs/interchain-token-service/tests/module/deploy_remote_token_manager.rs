use ethers_core::abi::AbiEncode;
use ethers_core::types::U256;
use gas_service::events::GasServiceEvent;
use gateway::events::GatewayEvent;
use interchain_token_transfer_gmp::{Bytes32, DeployTokenManager};
use solana_program::keccak::hash;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signature::Signer;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::transaction::Transaction;
use test_fixtures::test_setup::TestFixture;
use token_manager::TokenManagerType;

use crate::program_test;

#[tokio::test]
async fn test_deploy_remote_token_manager() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let gas_service_initial_saldo = fixture
        .banks_client
        .get_account(gas_service_root_pda)
        .await
        .unwrap()
        .unwrap()
        .lamports;

    let interchain_address_tracker_owner = Keypair::new();
    let trusted_chain_name = "ethereum".to_string();
    let trusted_chain_addr = "0x1234567890123456789012345678901234567890".to_string();
    let (associated_trusted_address, associated_trusted_address_from_account) = fixture
        .prepare_trusted_address_iatracker(
            interchain_address_tracker_owner,
            trusted_chain_name.clone(),
            trusted_chain_addr,
        )
        .await;

    let salt = [1u8; 32];
    let destination_chain = trusted_chain_name;
    let token_manager_type = TokenManagerType::LockUnlock;
    let params: Vec<u8> = vec![0, 1, 2, 3];
    let gas_value = 777; // fees

    let token_id = interchain_token_service::processor::Processor::interchain_token_id(
        &fixture.payer.pubkey(),
        salt,
    );
    let payload = DeployTokenManager {
        token_id: Bytes32(token_id),
        token_manager_type: U256::from(token_manager_type.clone() as u8),
        params: params.clone(),
    }
    .encode();
    let payload_hash = hash(&payload).to_bytes();

    // Action
    let recent_blockhash = fixture.refresh_blockhash().await;
    let transaction = Transaction::new_signed_with_payer(
        &[
            interchain_token_service::instruction::build_deploy_remote_token_manager_instruction(
                &fixture.payer.pubkey(),
                salt,
                destination_chain.clone(),
                token_manager_type.clone(),
                params,
                gas_value,
                &associated_trusted_address,
            )
            .unwrap(),
        ],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata { result, metadata } = fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    // Assert
    assert!(
        result.is_ok(),
        "falied to process deploy_remote_token_manager instruction"
    );

    // check if gas service got what it should / fees
    assert_eq!(
        fixture
            .banks_client
            .get_account(gas_service_root_pda)
            .await
            .unwrap()
            .unwrap()
            .lamports,
        gas_service_initial_saldo + gas_value
    );

    let metadata = metadata
        .clone()
        .ok_or("transaction does not have a metadata")
        .unwrap();

    let gas_service_event = metadata
        .log_messages
        .iter()
        .find_map(GasServiceEvent::parse_log);

    assert_eq!(
        gas_service_event,
        Some(GasServiceEvent::NativeGasPaidForContractCall {
            sender: fixture.payer.pubkey().into(),
            destination_chain: destination_chain.clone(),
            destination_address: associated_trusted_address_from_account.clone().into(),
            payload_hash,
            fees: gas_value,
            refund_address: fixture.payer.pubkey().into()
        })
    );

    let gateway_event = metadata
        .log_messages
        .iter()
        .find_map(GatewayEvent::parse_log);

    assert_eq!(
        gateway_event,
        Some(GatewayEvent::CallContract {
            sender: fixture.payer.pubkey().into(),
            destination_chain: destination_chain.as_bytes().to_vec(),
            destination_address: associated_trusted_address_from_account.into(),
            payload,
            payload_hash
        })
    );
}
