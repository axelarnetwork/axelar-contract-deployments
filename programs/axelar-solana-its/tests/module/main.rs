#![cfg(test)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::default_numeric_fallback,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::let_underscore_untyped,
    clippy::little_endian_bytes,
    clippy::missing_errors_doc,
    clippy::non_ascii_literal,
    clippy::panic,
    clippy::should_panic_without_expect,
    clippy::str_to_string,
    clippy::tests_outside_test_module,
    clippy::too_many_lines,
    clippy::unwrap_used,
    missing_docs,
    unused_must_use
)]

mod deploy_interchain_token;
mod deploy_manager_mismatch;
mod deploy_remote;
mod fee_handling;
mod flow_limits;
mod from_evm_to_solana;
mod from_solana_to_evm;
mod handover_mint_authority;
mod idempotent_ata_test;
mod memo_cpi_transfer;
mod metadata_length_validation;
mod metadata_retrieval;
mod pause_unpause;
mod role_management;
mod token_id_validation;
mod transfer_destination;

use solana_banks_interface::BanksTransactionResultWithSimulation;
use solana_program_test::BanksTransactionResultWithMetadata;
use solana_sdk::account::Account;
use solana_sdk::account_info::Account as AccountTrait;
use solana_sdk::account_info::IntoAccountInfo;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_error::ProgramError;
use solana_sdk::program_pack::Pack as _;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use test_context::AsyncTestContext;

use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::events::CallContractEvent;
use axelar_solana_gateway::state::incoming_message::command_id;
use axelar_solana_gateway_test_fixtures::base::workspace_root_dir;
use axelar_solana_gateway_test_fixtures::gas_service::GasServiceUtils;
use axelar_solana_gateway_test_fixtures::gateway::random_message;
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
use axelar_solana_its::instruction::ExecuteInstructionInputs;
use event_cpi_test_utils::get_first_event_cpi_occurrence;
use evm_contracts_test_suite::chain::TestBlockchain;
use evm_contracts_test_suite::ethers::abi::Detokenize;
use evm_contracts_test_suite::ethers::contract::{ContractCall, EthLogDecode, Event as EvmEvent};
use evm_contracts_test_suite::ethers::providers::Middleware;
use evm_contracts_test_suite::ethers::signers::Signer as _;
use evm_contracts_test_suite::ethers::types::{Bytes, TransactionReceipt};
use evm_contracts_test_suite::evm_contracts_rs::contracts::axelar_amplifier_gateway::{
    ContractCallFilter, Message as EvmAxelarMessage,
};
use evm_contracts_test_suite::evm_weighted_signers::WeightedSigners;
use evm_contracts_test_suite::{
    evm_weighted_signers, get_domain_separator, EvmSigner, ItsContracts,
};
use interchain_token_transfer_gmp::{GMPPayload, ReceiveFromHub};
use program_utils::pda::BorshPda;

const SOLANA_CHAIN_NAME: &str = "solana-localnet";
const EVM_CHAIN_NAME: &str = "ethereum";
const ITS_HUB_TRUSTED_CHAIN_NAME: &str = "axelar";
const ITS_HUB_TRUSTED_CONTRACT_ADDRESS: &str =
    "axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4";

pub struct ItsTestContext {
    pub solana_chain: SolanaAxelarIntegrationMetadata,
    pub solana_wallet: Pubkey,
    pub solana_gas_utils: GasServiceUtils,
    pub evm_chain: TestBlockchain,
    pub solana_chain_name: String,
    pub evm_chain_name: String,
    pub evm_its_contracts: ItsContracts,
    pub counter_pda: Pubkey,
    pub evm_signer: EvmSigner,
    pub evm_weighted_signers: WeightedSigners,
    pub evm_domain_separator: [u8; 32],
    pub deployed_interchain_token: [u8; 32],
}

impl AsyncTestContext for ItsTestContext {
    async fn setup() -> Self {
        let (mut solana_chain, counter_pda) = axelar_solana_setup().await;
        let (evm_chain, evm_signer, evm_its_contracts, evm_weighted_signers, evm_domain_separator) =
            axelar_evm_setup().await;
        let solana_gas_utils = solana_chain.fixture.deploy_gas_service().await;

        solana_chain
            .fixture
            .init_gas_config(&solana_gas_utils)
            .await
            .unwrap();

        let solana_wallet = solana_chain.fixture.payer.pubkey();

        let mut this = Self {
            solana_chain,
            solana_wallet,
            solana_gas_utils,
            evm_chain,
            solana_chain_name: SOLANA_CHAIN_NAME.to_string(),
            evm_chain_name: EVM_CHAIN_NAME.to_string(),
            evm_its_contracts,
            counter_pda,
            evm_signer,
            evm_weighted_signers,
            evm_domain_separator,
            deployed_interchain_token: [0; 32],
        };

        this.deploy_interchain_token().await;

        this
    }
}

impl ItsTestContext {
    #[allow(clippy::panic)]
    async fn relay_to_solana(
        &mut self,
        payload: &[u8],
        maybe_mint: Option<Pubkey>,
        token_program: Pubkey,
    ) -> (
        Vec<solana_sdk::inner_instruction::InnerInstruction>,
        BanksTransactionResultWithMetadata,
    ) {
        let payload = route_its_hub(
            GMPPayload::decode(payload).unwrap(),
            self.evm_chain_name.clone(),
        );

        let encoded_payload = payload.encode();
        let payload_hash = solana_sdk::keccak::hash(&encoded_payload).to_bytes();
        let message = random_hub_message_with_destination_and_payload(
            axelar_solana_its::id().to_string(),
            payload_hash,
        );

        let message_from_multisig_prover = self
            .solana_chain
            .sign_session_and_approve_messages(
                &self.solana_chain.signers.clone(),
                &[message.clone()],
            )
            .await
            .unwrap();

        let message_payload_pda = self
            .solana_chain
            .upload_message_payload(&message, &encoded_payload)
            .await
            .unwrap();

        // Action: set message status as executed by calling the destination program
        let (incoming_message_pda, ..) = axelar_solana_gateway::get_incoming_message_pda(
            &command_id(&message.cc_id.chain, &message.cc_id.id),
        );

        let merkelised_message = message_from_multisig_prover
            .iter()
            .find(|x| x.leaf.message.cc_id == message.cc_id)
            .unwrap()
            .clone();

        let its_ix_inputs = ExecuteInstructionInputs::builder()
            .payer(self.solana_chain.fixture.payer.pubkey())
            .incoming_message_pda(incoming_message_pda)
            .message_payload_pda(message_payload_pda)
            .message(merkelised_message.leaf.message)
            .payload(payload)
            .token_program(token_program)
            .mint_opt(maybe_mint)
            .build();

        let instruction = axelar_solana_its::instruction::execute(its_ix_inputs)
            .expect("failed to create instruction");

        // Simulate first to get inner_ixs for event extraction
        let simulation_result = self.simulate_solana_tx(&[instruction.clone()]).await;
        let inner_ixs = simulation_result
            .simulation_details
            .unwrap()
            .inner_instructions
            .unwrap()
            .first()
            .cloned()
            .unwrap_or_default();

        // Then execute the transaction
        let tx_result = match self.solana_chain.fixture.send_tx(&[instruction]).await {
            Ok(tx) | Err(tx) => tx,
        };

        (inner_ixs, tx_result)
    }

    pub async fn send_solana_tx(
        &mut self,
        ixs: &[Instruction],
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        self.solana_chain.fixture.send_tx(ixs).await
    }

    pub async fn simulate_solana_tx(
        &mut self,
        ixs: &[Instruction],
    ) -> BanksTransactionResultWithSimulation {
        self.solana_chain.fixture.simulate_tx(ixs).await.unwrap()
    }

    pub async fn send_solana_tx_with(
        &mut self,
        payer: &Keypair,
        ixs: &[Instruction],
        signers: &[Keypair],
    ) -> Result<BanksTransactionResultWithMetadata, BanksTransactionResultWithMetadata> {
        self.solana_chain
            .fixture
            .send_tx_with_custom(&payer.pubkey(), ixs, signers)
            .await
            .map(|x| x.1)
            .map_err(|x| x.1)
    }

    async fn relay_to_evm(&mut self, payload: &[u8]) {
        let payload = route_its_hub(
            GMPPayload::decode(payload).unwrap(),
            self.solana_chain_name.clone(),
        );

        let encoded_payload = payload.encode();
        let payload_hash = solana_sdk::keccak::hash(&encoded_payload).to_bytes();
        let message = EvmAxelarMessage {
            source_chain: ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            message_id: String::from_utf8_lossy(&payload_hash).to_string(),
            source_address: ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_string(),
            contract_address: self.evm_its_contracts.interchain_token_service.address(),
            payload_hash,
        };

        let approve_contract_call_command =
            evm_weighted_signers::get_approve_contract_call(message.clone());

        let proof = evm_weighted_signers::get_weighted_signatures_proof(
            &approve_contract_call_command,
            &mut self.evm_weighted_signers,
            self.evm_domain_separator,
        );

        call_evm(
            self.evm_its_contracts
                .gateway
                .approve_messages(vec![message.clone()], proof),
        )
        .await;

        let is_approved = self
            .evm_its_contracts
            .gateway
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

        let command_id = self
            .evm_its_contracts
            .gateway
            .message_to_command_id(
                ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
                message.message_id.clone(),
            )
            .await
            .unwrap();

        call_evm(self.evm_its_contracts.interchain_token_service.execute(
            command_id,
            message.source_chain,
            message.source_address,
            encoded_payload.into(),
        ))
        .await;
    }

    async fn deploy_interchain_token(&mut self) {
        let salt = solana_sdk::keccak::hash(b"TestTokenSalt").0;
        let deploy_local_ix = axelar_solana_its::instruction::deploy_interchain_token(
            self.solana_wallet,
            self.solana_wallet,
            salt,
            "Test Token".to_owned(),
            "TT".to_owned(),
            9,
            0,
            Some(self.solana_wallet),
        )
        .unwrap();

        // Simulate first to get the event
        let simulation_result = self.simulate_solana_tx(&[deploy_local_ix.clone()]).await;
        let inner_ixs = simulation_result
            .simulation_details
            .unwrap()
            .inner_instructions
            .unwrap()
            .first()
            .cloned()
            .unwrap();
        let deploy_event = get_first_event_cpi_occurrence::<
            axelar_solana_its::events::InterchainTokenDeployed,
        >(&inner_ixs)
        .unwrap();

        // Then execute the transaction
        let _tx = self.send_solana_tx(&[deploy_local_ix]).await.unwrap();

        assert_eq!(deploy_event.name, "Test Token", "token name does not match");

        let approve_remote_deployment =
            axelar_solana_its::instruction::approve_deploy_remote_interchain_token(
                self.solana_wallet,
                self.solana_wallet,
                self.solana_wallet,
                salt,
                self.evm_chain_name.clone(),
                self.evm_signer.wallet.address().as_bytes().to_vec(),
            )
            .unwrap();

        self.send_solana_tx(&[approve_remote_deployment])
            .await
            .unwrap();

        let deploy_remote_ix =
            axelar_solana_its::instruction::deploy_remote_interchain_token_with_minter(
                self.solana_wallet,
                self.solana_wallet,
                salt,
                self.solana_wallet,
                self.evm_chain_name.clone(),
                self.evm_signer.wallet.address().as_bytes().to_vec(),
                0,
            )
            .unwrap();

        // Simulate first to get the event
        let simulation_result = self.simulate_solana_tx(&[deploy_remote_ix.clone()]).await;
        let inner_ixs = simulation_result
            .simulation_details
            .unwrap()
            .inner_instructions
            .unwrap()
            .first()
            .cloned()
            .unwrap();
        let call_contract_event =
            event_cpi_test_utils::get_first_event_cpi_occurrence::<CallContractEvent>(&inner_ixs)
                .expect("CallContractEvent not found in inner instructions");

        // Then execute the transaction
        self.send_solana_tx(&[deploy_remote_ix]).await.unwrap();

        self.relay_to_evm(&call_contract_event.payload).await;

        let log = retrieve_evm_log_with_filter(
            self.evm_its_contracts
                .interchain_token_service
                .interchain_token_deployed_filter(),
        )
        .await;

        let expected_token_id = axelar_solana_its::interchain_token_id(&self.solana_wallet, &salt);

        assert_eq!(log.token_id, expected_token_id, "token_id does not match");

        self.deployed_interchain_token = expected_token_id;
    }

    async fn test_interchain_transfer(
        &mut self,
        token_id: [u8; 32],
        solana_token: Pubkey,
        initial_balance: u64,
        token_account: Pubkey,
    ) {
        let amount = 100;

        let transfer_ix = axelar_solana_its::instruction::interchain_transfer(
            self.solana_wallet,
            self.solana_wallet,
            token_account,
            token_id,
            self.evm_chain_name.clone(),
            self.evm_signer.wallet.address().as_bytes().to_vec(),
            amount,
            solana_token,
            spl_token_2022::id(),
            0,
        )
        .unwrap();

        // Simulate first to get events
        let simulation_result = self.simulate_solana_tx(&[transfer_ix.clone()]).await;
        let inner_ixs = simulation_result
            .simulation_details
            .unwrap()
            .inner_instructions
            .unwrap()
            .first()
            .cloned()
            .unwrap();

        let call_contract_event =
            event_cpi_test_utils::get_first_event_cpi_occurrence::<CallContractEvent>(&inner_ixs)
                .expect("CallContractEvent not found in inner instructions");

        // Find the InterchainTransfer event
        let transfer_event = get_first_event_cpi_occurrence::<
            axelar_solana_its::events::InterchainTransfer,
        >(&inner_ixs)
        .expect("InterchainTransfer event not found");

        assert_eq!(transfer_event.amount, amount);

        // Then execute the transaction
        self.send_solana_tx(&[transfer_ix]).await.unwrap();

        self.relay_to_evm(&call_contract_event.payload).await;

        let log = retrieve_evm_log_with_filter(
            self.evm_its_contracts
                .interchain_token_service
                .interchain_transfer_received_filter(),
        )
        .await;

        assert_eq!(log.amount, amount.into());
        let amount_back = amount - 50;

        self.evm_its_contracts
            .interchain_token_service
            .interchain_transfer(
                token_id,
                self.solana_chain_name.clone(),
                self.solana_wallet.to_bytes().into(),
                amount_back.into(),
                Bytes::new(),
                0.into(),
            )
            .send()
            .await
            .unwrap()
            .await
            .unwrap();

        let log = retrieve_evm_log_with_filter(
            self.evm_its_contracts
                .interchain_token_service
                .interchain_transfer_filter(),
        )
        .await;

        assert_eq!(log.amount, amount_back.into());

        let log: ContractCallFilter = self
            .evm_its_contracts
            .gateway
            .contract_call_filter()
            .query()
            .await
            .unwrap()
            .into_iter()
            .next()
            .ok_or("no logs found")
            .unwrap();

        let (inner_ixs, _tx) = self
            .relay_to_solana(
                log.payload.as_ref(),
                Some(solana_token),
                spl_token_2022::id(),
            )
            .await;
        let transfer_received_event = get_first_event_cpi_occurrence::<
            axelar_solana_its::events::InterchainTransferReceived,
        >(&inner_ixs)
        .unwrap();
        assert_eq!(transfer_received_event.amount, amount_back);

        let token_account_data = self
            .solana_chain
            .try_get_account_no_checks(&token_account)
            .await
            .unwrap()
            .unwrap()
            .data;

        let account =
            spl_token_2022::state::Account::unpack_from_slice(&token_account_data).unwrap();

        assert_eq!(
            account.amount,
            initial_balance - amount + amount_back,
            "New balance doesn't match expected balance"
        );
    }
}

async fn axelar_solana_setup() -> (SolanaAxelarIntegrationMetadata, Pubkey) {
    let programs = vec![
        ("axelar_solana_its.so".into(), axelar_solana_its::id()),
        (
            workspace_root_dir()
                .join("programs")
                .join("axelar-solana-its")
                .join("tests")
                .join("mpl_token_metadata.so"),
            mpl_token_metadata::ID,
        ),
        (
            "axelar_solana_memo_program.so".into(),
            axelar_solana_memo_program::id(),
        ),
    ];

    let mut solana_chain = SolanaAxelarIntegration::builder()
        .initial_signer_weights(vec![555, 222])
        .programs_to_deploy(programs)
        .build()
        .setup()
        .await;

    let (counter_pda, counter_bump) = axelar_solana_memo_program::get_counter_pda();

    let _metadata = solana_chain
        .fixture
        .send_tx(&[axelar_solana_memo_program::instruction::initialize(
            &solana_chain.fixture.payer.pubkey(),
            &(counter_pda, counter_bump),
        )
        .unwrap()])
        .await;

    let _metadata = solana_chain
        .fixture
        .send_tx_with_custom_signers(
            &[
                system_instruction::transfer(
                    &solana_chain.fixture.payer.pubkey(),
                    &solana_chain.upgrade_authority.pubkey(),
                    u32::MAX.into(),
                ),
                axelar_solana_its::instruction::initialize(
                    solana_chain.upgrade_authority.pubkey(),
                    solana_chain.fixture.payer.pubkey(),
                    SOLANA_CHAIN_NAME.to_owned(),
                    ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_owned(),
                )
                .unwrap(),
                axelar_solana_its::instruction::set_trusted_chain(
                    solana_chain.fixture.payer.pubkey(),
                    solana_chain.upgrade_authority.pubkey(),
                    EVM_CHAIN_NAME.to_owned(),
                )
                .unwrap(),
            ],
            &[
                &solana_chain.upgrade_authority.insecure_clone(),
                &solana_chain.fixture.payer.insecure_clone(),
            ],
        )
        .await;

    (solana_chain, counter_pda)
}

async fn axelar_evm_setup() -> (
    TestBlockchain,
    EvmSigner,
    ItsContracts,
    WeightedSigners,
    [u8; 32],
) {
    let evm_chain = evm_contracts_test_suite::chain::TestBlockchain::new();
    let alice = evm_chain.construct_provider_with_signer(0);
    let operators1 = evm_weighted_signers::create_operator_set(&evm_chain, 0..5);
    let operators2 = evm_weighted_signers::create_operator_set(&evm_chain, 5..9);
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
        .last()
        .expect("no logs found")
}

async fn call_evm<M, D>(contract_call: ContractCall<M, D>) -> TransactionReceipt
where
    M: Middleware,
    D: Detokenize,
{
    contract_call.send().await.unwrap().await.unwrap().unwrap()
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
