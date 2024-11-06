#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::str_to_string
)]

mod deploy_interchain_token;
mod deploy_token_manager;
mod from_solana_to_evm;
mod initialize;
mod its_gmp_payload;

use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{get_domain_separator, ItsContracts};
use interchain_token_transfer_gmp::{GMPPayload, ReceiveFromHub};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::test_setup::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};

mod from_evm_to_solana;

const SOLANA_CHAIN_NAME: &str = "solana-localnet";
const ITS_CHAIN_NAME: &str = "axelar";

pub struct ItsProgramWrapper {
    pub solana_chain: SolanaAxelarIntegrationMetadata,
    pub chain_name: String,
    pub counter_pda: Option<Pubkey>,
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

async fn axelar_solana_setup(with_memo: bool) -> ItsProgramWrapper {
    let mut programs = vec![("axelar_solana_its.so".into(), axelar_solana_its::id())];
    if with_memo {
        programs.push((
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        ));
    }

    let mut solana_chain = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(programs)
        .build()
        .setup()
        .await;

    #[allow(clippy::if_then_some_else_none)] // bool.then() doesn´t allow async
    let counter_pda = if with_memo {
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

        Some(counter_pda)
    } else {
        None
    };

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
        chain_name: SOLANA_CHAIN_NAME.to_string(),
        counter_pda,
    }
}

fn prepare_receive_from_hub(payload: &GMPPayload, source_chain: String) -> GMPPayload {
    GMPPayload::ReceiveFromHub(ReceiveFromHub {
        selector: ReceiveFromHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        source_chain,
        payload: payload.encode().into(),
    })
}

#[allow(clippy::panic)]
fn route_its_hub(payload: GMPPayload, source_chain: String) -> GMPPayload {
    let GMPPayload::SendToHub(inner) = payload else {
        panic!("Expected SendToHub payload");
    };

    GMPPayload::ReceiveFromHub(ReceiveFromHub {
        selector: ReceiveFromHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        payload: inner.payload,
        source_chain,
    })
}

async fn axelar_evm_setup() -> (
    TestBlockchain,
    evm_contracts_test_suite::EvmSigner,
    ItsContracts,
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

    let its_contracts = alice
        .deploy_all_its(
            alice.wallet.address(),
            alice.wallet.address(),
            &[operators1, operators2.clone()],
            [SOLANA_CHAIN_NAME.to_string()],
        )
        .await
        .unwrap();

    its_contracts
        .interchain_token_service
        .set_trusted_address(SOLANA_CHAIN_NAME.to_owned(), "hub".to_owned())
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    its_contracts
        .interchain_token_service
        .set_trusted_address(ITS_CHAIN_NAME.to_owned(), "some address".to_owned())
        .send()
        .await
        .unwrap()
        .await
        .unwrap();

    (
        evm_chain,
        alice,
        its_contracts,
        operators2,
        get_domain_separator(),
    )
}
