use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway;
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{get_domain_separator, ContractMiddleware};
use solana_program_test::{processor, ProgramTest};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
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

async fn axelar_solana_setup() -> (
    TestFixture,
    solana_sdk::pubkey::Pubkey,
    Vec<TestSigner>,
    Pubkey,
) {
    let mut fixture = TestFixture::new(program_test()).await;
    let signers = vec![
        create_signer_with_weight(10_u128).unwrap(),
        create_signer_with_weight(4_u128).unwrap(),
    ];
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module(&signers),
            Pubkey::new_unique(),
        )
        .await;
    let (counter_pda, counter_bump) =
        axelar_solana_memo_program::get_counter_pda(&gateway_root_pda);
    fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &fixture.payer.pubkey(),
            &gateway_root_pda,
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    (fixture, gateway_root_pda, signers, counter_pda)
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
