use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::evm_contracts_rs::contracts::{axelar_auth_weighted, axelar_gateway};
use evm_contracts_test_suite::evm_operators::OperatorSet;
use evm_contracts_test_suite::ContractMiddleware;
use solana_program_test::{processor, ProgramTest};
use test_fixtures::execute_data::{create_signer_with_weight, TestSigner};
use test_fixtures::test_setup::TestFixture;

mod from_evm_to_solana;
mod from_solana_to_evm;

pub fn program_test() -> ProgramTest {
    let mut pt = ProgramTest::new(
        "gmp_gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    );

    pt.add_program(
        "axelar_solana_memo_program",
        axelar_solana_memo_program::id(),
        processor!(axelar_solana_memo_program::processor::process_instruction),
    );

    pt
}

async fn axelar_solana_setup() -> (TestFixture, solana_sdk::pubkey::Pubkey, Vec<TestSigner>) {
    let mut fixture = TestFixture::new(program_test()).await;
    let operators = vec![
        create_signer_with_weight(10).unwrap(),
        create_signer_with_weight(4).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(fixture.init_auth_weighted_module(&operators))
        .await;
    (fixture, gateway_root_pda, operators)
}

async fn axelar_evm_setup() -> (
    TestBlockchain,
    evm_contracts_test_suite::EvmSigner,
    axelar_auth_weighted::AxelarAuthWeighted<ContractMiddleware>,
    axelar_gateway::AxelarGateway<ContractMiddleware>,
    OperatorSet,
) {
    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let operators1 = evm_contracts_test_suite::evm_operators::create_operator_set(&evm_chain, 0..5);
    let operators2 = evm_contracts_test_suite::evm_operators::create_operator_set(&evm_chain, 5..9);
    let evm_aw = alice
        .deploy_axelar_auth_weighted(&[operators1, operators2.clone()])
        .await
        .unwrap();
    let evm_gateway = alice.deploy_axelar_gateway(&evm_aw).await.unwrap();

    (evm_chain, alice, evm_aw, evm_gateway, operators2)
}
