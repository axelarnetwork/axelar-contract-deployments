use gateway::accounts::GatewayConfig;
use interchain_token_transfer_gmp::ethers_core::types::{Address, U256};
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, GMPPayload, InterchainTransfer};
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, ProgramFailedToComplete))")]
async fn test_interchain_transfer() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let gateway_operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(GatewayConfig::new(
            0,
            fixture.init_operators_and_epochs(&gateway_operators),
        ))
        .await;
    let gas_service_root_pda = fixture.init_gas_service().await;
    let interchain_token_service_root_pda = fixture
        .init_its_root_pda(&gateway_root_pda, &gas_service_root_pda)
        .await;
    let deploy_token_manager_messages = [(
        test_fixtures::axelar_message::message().unwrap(),
        interchain_token_service_root_pda,
    )];
    let gateway_approved_message_pda = fixture
        .fully_approve_messages(
            &gateway_root_pda,
            &deploy_token_manager_messages,
            gateway_operators,
        )
        .await[0];

    // ACTION: deploy interchain token
    let ix = interchain_token_service::instruction::build_execute_instruction(
        &gateway_approved_message_pda,
        &interchain_token_service_root_pda,
        &gateway_root_pda,
        &gas_service_root_pda,
        &[],
        GMPPayload::InterchainTransfer(InterchainTransfer {
            token_id: Bytes32(keccak256("random-token-id")),
            source_address: Address::random().0.to_vec(),
            destination_address: Address::random().0.to_vec(),
            amount: U256::from(100),
            data: vec![],
        }),
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&fixture.payer.pubkey()),
        &[&fixture.payer],
        fixture.banks_client.get_latest_blockhash().await.unwrap(),
    );
    fixture
        .banks_client
        .process_transaction(transaction)
        .await
        .unwrap();

    // Assert
}
