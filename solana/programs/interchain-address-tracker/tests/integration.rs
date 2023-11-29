use interchain_address_tracker::get_associated_chain_address;
use solana_program::{program_pack::Pack, pubkey::Pubkey};
use solana_program_test::{processor, tokio, ProgramTest};
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;

fn program_test() -> ProgramTest {
    ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace("-", "_"),
        interchain_address_tracker::id(),
        processor!(interchain_address_tracker::processor::Processor::process_instruction),
    )
}

#[tokio::test]
async fn test_create_and_store_chain_name() {
    let wallet_address = Pubkey::new_unique();
    let associated_chain_address = get_associated_chain_address(&wallet_address);
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;

    let rent = banks_client.get_rent().await.unwrap();
    let expected_token_account_len = interchain_address_tracker::state::Account::LEN;
    let expected_token_account_balance = rent.minimum_balance(expected_token_account_len);

    // Associated account does not exist
    assert_eq!(
        banks_client
            .get_account(associated_chain_address)
            .await
            .expect("get_account"),
        None,
    );

    let ix = interchain_address_tracker::instruction::build_create_registered_chain_instruction(
        &payer.pubkey(),
        &associated_chain_address,
        &wallet_address,
        "MyChainABC".to_string(),
    )
    .unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await.unwrap();

    // Associated account now exists
    let associated_account = banks_client
        .get_account(associated_chain_address)
        .await
        .expect("get_account")
        .expect("associated_account not none");

    assert_eq!(associated_account.owner, interchain_address_tracker::id());
    assert_eq!(
        associated_account.data.len(),
        interchain_address_tracker::state::Account::LEN
    );
    assert_eq!(associated_account.lamports, expected_token_account_balance);

    let account_info = interchain_address_tracker::state::Account::unpack_from_slice(
        associated_account.data.as_slice(),
    )
    .unwrap();
    assert_eq!(account_info.chain_name, "MyChainABC".to_string());
    assert_eq!(account_info.owner, wallet_address);
}
