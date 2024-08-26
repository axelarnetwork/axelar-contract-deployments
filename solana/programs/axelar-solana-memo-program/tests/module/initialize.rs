use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::state::Counter;
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
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    // Assert
    let counter_pda = solana_chain
        .fixture
        .get_account::<Counter>(&counter_pda, &axelar_solana_memo_program::id())
        .await;
    assert_eq!(counter_pda.bump, counter_bump);
    assert_eq!(counter_pda.counter, 0);
}
