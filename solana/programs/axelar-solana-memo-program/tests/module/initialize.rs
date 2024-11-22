use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::state::Counter;
use borsh::BorshDeserialize;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
async fn test_initialize() {
    // Setup
    let mut solana_chain = program_test().await;

    // Action
    let (counter_pda, counter_bump) = get_counter_pda(&solana_chain.gateway_root_pda);
    let initialize = axelar_solana_memo_program::instruction::initialize(
        &solana_chain.fixture.payer.pubkey().clone(),
        &solana_chain.gateway_root_pda.clone(),
        &(counter_pda, counter_bump),
    )
    .unwrap();
    solana_chain.send_tx(&[initialize]).await.unwrap();

    // Assert
    let counter_pda = solana_chain
        .fixture
        .get_account(&counter_pda, &axelar_solana_memo_program::id())
        .await;
    let counter_pda = Counter::try_from_slice(&counter_pda.data).unwrap();
    assert_eq!(counter_pda.bump, counter_bump);
    assert_eq!(counter_pda.counter, 0);
}
