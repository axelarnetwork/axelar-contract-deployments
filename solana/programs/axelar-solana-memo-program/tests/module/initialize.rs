use axelar_solana_memo_program::get_counter_pda;
use axelar_solana_memo_program::state::Counter;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;
use test_fixtures::execute_data::create_signer_with_weight;
use test_fixtures::test_setup::TestFixture;

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
async fn test_initialize() {
    // Setup
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;

    // Action
    let (counter_pda, counter_bump) = get_counter_pda(&gateway_root_pda);
    fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &fixture.payer.pubkey(),
            &gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    // Assert
    let counter_pda = fixture
        .get_account::<Counter>(&counter_pda, &axelar_solana_memo_program::id())
        .await;
    assert_eq!(counter_pda.bump, counter_bump);
    assert_eq!(counter_pda.counter, 0);
}
