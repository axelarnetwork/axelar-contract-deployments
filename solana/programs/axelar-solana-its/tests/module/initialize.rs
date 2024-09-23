#![cfg(feature = "test-bpf")]
use axelar_solana_its::state::InterchainTokenService;
use solana_program_test::tokio;
use solana_sdk::signature::Signer;

use crate::program_test;

#[rstest::rstest]
#[tokio::test]
async fn test_initialize() {
    let mut solana_chain = program_test().await;
    let (its_root_pda, its_root_pda_bump) =
        axelar_solana_its::its_root_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(its_root_pda, its_root_pda_bump),
        )
        .unwrap()])
        .await;

    let its_root_pda = solana_chain
        .fixture
        .get_account::<InterchainTokenService>(&its_root_pda, &axelar_solana_its::id())
        .await;

    assert_eq!(its_root_pda.bump, its_root_pda_bump);
}
