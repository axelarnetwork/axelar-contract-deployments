// #![cfg(feature = "test-sbf")]

use std::error::Error;

use solana_program::pubkey::Pubkey;
use solana_program_test::{tokio, BanksTransactionResultWithMetadata};
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

use crate::utils::program_test;

#[tokio::test]
async fn test_initialize() -> Result<(), Box<dyn Error>> {
    let (mut banks_client, payer, recent_blockhash) = program_test().start().await;
    let (root_pda, _bump) = Pubkey::find_program_address(&[&[]], &gateway::id());

    let payload = b"All you need is potatoes!";

    let ix = gateway::instruction::build_initialize_ix(payer.pubkey(), root_pda, payload)?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    let BanksTransactionResultWithMetadata {
        result,
        metadata: _,
    } = banks_client.process_transaction_with_metadata(tx).await?;

    assert!({ result.is_ok() });

    let actual_data = banks_client
        .get_account(root_pda)
        .await
        .expect("get_account")
        .unwrap();

    // [2..] as the data serialized to account consist of 2-byte len descriptor.
    assert_eq!(actual_data.owner, gateway::id());
    assert_eq!(actual_data.data[2..].len(), payload.len());
    assert_eq!(&actual_data.data[2..], payload);

    Ok(())
}
