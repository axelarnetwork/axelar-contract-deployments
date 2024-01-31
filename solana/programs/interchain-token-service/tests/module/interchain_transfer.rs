use interchain_token_transfer_gmp::ethers_core::types::{Address, U256};
use interchain_token_transfer_gmp::ethers_core::utils::keccak256;
use interchain_token_transfer_gmp::{Bytes32, GMPPayload, InterchainTransfer};
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

#[tokio::test]
#[should_panic(expected = "TransactionError(InstructionError(0, ProgramFailedToComplete))")]
async fn test_deploy_interchain_token() {
    let mut fixture = super::utils::TestFixture::new().await;

    let ix = interchain_token_service::instruction::build_execute_instruction(
        &fixture.payer.pubkey(),
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
}
