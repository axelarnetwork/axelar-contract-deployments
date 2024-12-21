#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
    clippy::panic,
    clippy::str_to_string,
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    unused_must_use
)]

mod deploy_interchain_token;
mod deploy_token_manager;
mod flow_limits;
mod from_evm_to_solana;
mod from_solana_to_evm;
mod its_gmp_payload;
mod pause_unpause;
mod role_management;

use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::processor::GatewayEvent;
use axelar_solana_gateway::state::incoming_message::command_id;
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, random_message, ProgramInvocationState,
};
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use axelar_solana_its::instructions::ItsGmpInstructionInputs;
use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::ethers::abi::Detokenize;
use evm_contracts_test_suite::ethers::contract::{ContractCall, EthLogDecode, Event as EvmEvent};
use evm_contracts_test_suite::ethers::providers::Middleware;
use evm_contracts_test_suite::ethers::types::{Address, TransactionReceipt};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::{
    AxelarAmplifierGateway as EvmAxelarAmplifierGateway, Message as EvmAxelarMessage,
    Proof as EvmAxelarProof,
};
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{
    evm_weighted_signers, get_domain_separator, ContractMiddleware, ItsContracts,
};
use interchain_token_transfer_gmp::{GMPPayload, ReceiveFromHub};
use program_utils::BorshPda;
use solana_program_test::BanksTransactionResultWithMetadata;
use solana_sdk::account::Account;
use solana_sdk::account_info::Account as AccountTrait;
use solana_sdk::account_info::IntoAccountInfo;
use solana_sdk::clock::Clock;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

const SOLANA_CHAIN_NAME: &str = "solana-localnet";
const ITS_HUB_TRUSTED_CHAIN_NAME: &str = "axelar";
const ITS_HUB_TRUSTED_CONTRACT_ADDRESS: &str =
    "axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4";

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

        let _metadata = solana_chain
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

    let _metadata = solana_chain
        .fixture
        .send_tx(&[axelar_solana_its::instructions::initialize(
            solana_chain.fixture.payer.pubkey(),
            solana_chain.gateway_root_pda,
            solana_chain.fixture.payer.pubkey(),
        )
        .unwrap()])
        .await;

    ItsProgramWrapper {
        solana_chain,
        chain_name: SOLANA_CHAIN_NAME.to_string(),
        counter_pda,
    }
}

#[must_use]
pub fn random_hub_message_with_destination_and_payload(
    destination_address: String,
    payload_hash: [u8; 32],
) -> Message {
    let mut message = random_message();
    message.destination_address = destination_address;
    message.payload_hash = payload_hash;
    message.source_address = ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_string();
    message
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

#[allow(clippy::panic)]
async fn relay_to_solana(
    payload: Vec<u8>,
    solana_chain: &mut SolanaAxelarIntegrationMetadata,
    maybe_mint: Option<Pubkey>,
    token_program: Pubkey,
) -> BanksTransactionResultWithMetadata {
    let solana_payload = GMPPayload::ReceiveFromHub(ReceiveFromHub {
        selector: ReceiveFromHub::MESSAGE_TYPE_ID.try_into().unwrap(),
        source_chain: "ethereum".to_owned(),
        payload: payload.into(),
    });
    let encoded_payload = solana_payload.encode();
    let payload_hash = solana_sdk::keccak::hash(&encoded_payload).to_bytes();
    let message = random_hub_message_with_destination_and_payload(
        axelar_solana_its::id().to_string(),
        payload_hash,
    );

    let message_from_multisig_prover = solana_chain
        .sign_session_and_approve_messages(&solana_chain.signers.clone(), &[message.clone()])
        .await
        .unwrap();

    let message_payload_pda = solana_chain
        .upload_message_payload(&message, &encoded_payload)
        .await
        .unwrap();

    // Action: set message status as executed by calling the destination program
    let (incoming_message_pda, ..) = axelar_solana_gateway::get_incoming_message_pda(&command_id(
        &message.cc_id.chain,
        &message.cc_id.id,
    ));

    let merkelised_message = message_from_multisig_prover
        .iter()
        .find(|x| x.leaf.message.cc_id == message.cc_id)
        .unwrap()
        .clone();

    let clock_sysvar: Clock = solana_chain
        .fixture
        .banks_client
        .get_sysvar()
        .await
        .unwrap();

    let its_ix_inputs = ItsGmpInstructionInputs::builder()
        .payer(solana_chain.fixture.payer.pubkey())
        .incoming_message_pda(incoming_message_pda)
        .message_payload_pda(message_payload_pda)
        .message(merkelised_message.leaf.message)
        .payload(solana_payload)
        .token_program(token_program)
        .timestamp(clock_sysvar.unix_timestamp)
        .mint_opt(maybe_mint)
        .build();

    let instruction = axelar_solana_its::instructions::its_gmp_payload(its_ix_inputs)
        .expect("failed to create instruction");

    match solana_chain.fixture.send_tx(&[instruction]).await {
        Ok(tx) | Err(tx) => tx,
    }
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
        .set_trusted_address(
            ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_owned(),
        )
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

async fn retrieve_evm_log_with_filter<M, T>(filter: EvmEvent<std::sync::Arc<M>, M, T>) -> T
where
    M: Middleware,
    T: EthLogDecode,
{
    filter
        .from_block(0_u64)
        .query()
        .await
        .unwrap()
        .into_iter()
        .next()
        .expect("no logs found")
}

async fn call_evm<M, D>(contract_call: ContractCall<M, D>) -> TransactionReceipt
where
    M: Middleware,
    D: Detokenize,
{
    contract_call.send().await.unwrap().await.unwrap().unwrap()
}

async fn ensure_evm_gateway_approval(
    message: EvmAxelarMessage,
    proof: EvmAxelarProof,
    gateway: &EvmAxelarAmplifierGateway<ContractMiddleware>,
) -> [u8; 32] {
    call_evm(gateway.approve_messages(vec![message.clone()], proof)).await;

    let is_approved = gateway
        .is_message_approved(
            ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            message.message_id.clone(),
            message.source_address.clone(),
            message.contract_address,
            message.payload_hash,
        )
        .await
        .unwrap();

    assert!(is_approved, "contract call was not approved");

    gateway
        .message_to_command_id(
            ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            message.message_id.clone(),
        )
        .await
        .unwrap()
}

fn prepare_evm_approve_contract_call(
    payload_hash: [u8; 32],
    destination_address: Address,
    signer_set: &mut evm_weighted_signers::WeightedSigners,
    domain_separator: [u8; 32],
) -> (Vec<EvmAxelarMessage>, EvmAxelarProof) {
    // TODO: use address from the contract call once we have the trusted addresses
    // in place (the address is currently empty)
    let message = EvmAxelarMessage {
        source_chain: ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
        message_id: String::from_utf8_lossy(&payload_hash).to_string(),
        source_address: ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_string(),
        contract_address: destination_address,
        payload_hash,
    };

    let approve_contract_call_command =
        evm_weighted_signers::get_approve_contract_call(message.clone());

    // Build command batch
    let signed_weighted_execute_input = evm_weighted_signers::get_weighted_signatures_proof(
        &approve_contract_call_command,
        signer_set,
        domain_separator,
    );

    (vec![message], signed_weighted_execute_input)
}

async fn call_solana_gateway(
    solana_fixture: &mut TestFixture,
    ix: Instruction,
) -> Vec<ProgramInvocationState<GatewayEvent>> {
    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&solana_fixture.payer.pubkey()),
        &[&solana_fixture.payer],
        solana_fixture
            .banks_client
            .get_latest_blockhash()
            .await
            .unwrap(),
    );
    let tx = solana_fixture
        .banks_client
        .process_transaction_with_metadata(transaction)
        .await
        .unwrap();

    assert!(tx.result.is_ok(), "transaction failed");
    get_gateway_events(&tx)
}

pub trait BorshPdaAccount: AccountTrait {
    fn deserialize<'a, T>(&'a mut self, key: &'a Pubkey) -> Result<T, ProgramError>
    where
        T: BorshPda;
}

impl BorshPdaAccount for Account {
    fn deserialize<'a, T>(&'a mut self, key: &'a Pubkey) -> Result<T, ProgramError>
    where
        T: BorshPda,
    {
        let acc_info = (key, self).into_account_info();

        T::load(&acc_info)
    }
}
