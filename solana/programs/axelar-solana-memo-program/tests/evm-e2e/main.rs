use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway;
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{get_domain_separator, ContractMiddleware};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};

mod from_evm_to_solana;
mod from_solana_to_evm;

pub struct MemoProgramWrapper {
    pub solana_chain: SolanaAxelarIntegrationMetadata,
    pub counter_pda: Pubkey,
}

async fn axelar_solana_setup() -> MemoProgramWrapper {
    let mut solana_chain = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(vec![(
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        )])
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

    MemoProgramWrapper {
        solana_chain,
        counter_pda,
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
