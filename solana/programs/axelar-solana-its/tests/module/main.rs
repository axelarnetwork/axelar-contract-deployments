#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::str_to_string
)]

mod initialize;
mod its_gmp_payload;

use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::ItsContracts;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};

mod from_evm_to_solana;

const SOLANA_CHAIN_NAME: &str = "solana-devnet";

pub struct ItsProgramWrapper {
    pub solana_chain: SolanaAxelarIntegrationMetadata,
    pub its_pda: Pubkey,
    pub chain_name: String,
}

pub async fn program_test() -> SolanaAxelarIntegrationMetadata {
    SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_its.so".into(),
            axelar_solana_its::id(),
        )])
        .build()
        .setup()
        .await
}

async fn axelar_solana_setup() -> ItsProgramWrapper {
    let mut solana_chain = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_its.so".into(),
            axelar_solana_its::id(),
        )])
        .build()
        .setup()
        .await;
    let (its_pda, its_pda_bump) =
        axelar_solana_its::find_its_root_pda(&solana_chain.gateway_root_pda);
    solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &solana_chain.gateway_root_pda,
            &(its_pda, its_pda_bump),
        )
        .unwrap()])
        .await;

    ItsProgramWrapper {
        solana_chain,
        its_pda,
        chain_name: SOLANA_CHAIN_NAME.to_string(),
    }
}

async fn axelar_evm_setup() -> (
    TestBlockchain,
    evm_contracts_test_suite::EvmSigner,
    ItsContracts,
) {
    use evm_contracts_test_suite::ethers::signers::Signer;

    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let operators1 =
        evm_contracts_test_suite::evm_weighted_signers::create_operator_set(&evm_chain, 0..5);
    let operators2 =
        evm_contracts_test_suite::evm_weighted_signers::create_operator_set(&evm_chain, 5..9);

    let its_contracts = alice
        .deploy_all_its(
            alice.wallet.address(),
            alice.wallet.address(),
            &[operators1, operators2],
            [SOLANA_CHAIN_NAME.to_string()],
        )
        .await
        .unwrap();

    (evm_chain, alice, its_contracts)
}
