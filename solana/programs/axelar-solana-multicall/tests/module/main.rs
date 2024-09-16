#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::str_to_string
)]

use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway;
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{get_domain_separator, ContractMiddleware};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};

mod from_evm_to_solana;
mod to_solana;

pub struct TestContext {
    pub solana_chain: SolanaAxelarIntegrationMetadata,
    pub memo_program_counter_pda: Pubkey,
}

async fn axelar_solana_setup() -> TestContext {
    let programs_to_deploy = vec![
        (
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        ),
        (
            "axelar_solana_multicall.so".into(),
            axelar_solana_multicall::id(),
        ),
    ];

    let mut solana_chain = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(programs_to_deploy)
        .build()
        .setup()
        .await;

    let (counter_pda, counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&solana_chain.gateway_root_pda);

    solana_chain
        .fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    TestContext {
        solana_chain,
        memo_program_counter_pda: counter_pda,
    }
}

async fn axelar_evm_setup() -> (
    TestBlockchain,
    evm_contracts_test_suite::EvmSigner,
    axelar_amplifier_gateway::AxelarAmplifierGateway<ContractMiddleware>,
    WeightedSigners,
    [u8; 32],
) {
    use evm_contracts_test_suite::ethers::signers::Signer;

    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let operators1 =
        evm_contracts_test_suite::evm_weighted_signers::create_operator_set(&evm_chain, 0..5);
    let operators2 =
        evm_contracts_test_suite::evm_weighted_signers::create_operator_set(&evm_chain, 5..9);
    let evm_gateway = alice
        .deploy_axelar_amplifier_gateway(
            &[operators1, operators2.clone()],
            alice.wallet.address(),
            alice.wallet.address(),
        )
        .await
        .unwrap();

    (
        evm_chain,
        alice,
        evm_gateway,
        operators2,
        get_domain_separator(),
    )
}
