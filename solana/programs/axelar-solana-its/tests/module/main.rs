#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
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
use axelar_solana_gateway_test_fixtures::base::TestFixture;
use axelar_solana_gateway_test_fixtures::gateway::{
    get_gateway_events, random_message, ProgramInvocationState,
};
use axelar_solana_gateway_test_fixtures::{
    SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata,
};
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
use solana_sdk::account::Account;
use solana_sdk::account_info::Account as AccountTrait;
use solana_sdk::account_info::IntoAccountInfo;
use solana_sdk::instruction::Instruction;
use solana_sdk::program_error::ProgramError;
use solana_sdk::program_pack::Pack;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::system_instruction;
use solana_sdk::transaction::Transaction;
use spl_token_2022::extension::ExtensionType;
use spl_token_2022::state::Mint;

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

pub trait TokenUtils {
    fn init_new_mint(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        decimals: u8,
    ) -> impl core::future::Future<Output = Pubkey> + Send;

    #[allow(clippy::too_many_arguments)]
    fn init_new_mint_with_fee(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
        transfer_fee_config_authority: Option<&Pubkey>,
        withdraw_withheld_authority: Option<&Pubkey>,
    ) -> impl core::future::Future<Output = Pubkey> + Send;

    fn mint_tokens_to(
        &mut self,
        mint: Pubkey,
        to: Pubkey,
        mint_authority: Keypair,
        amount: u64,
        token_program_id: Pubkey,
    ) -> impl core::future::Future<Output = ()> + Send;
}

impl TokenUtils for TestFixture {
    async fn init_new_mint(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        decimals: u8,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(Mint::LEN),
                    Mint::LEN.try_into().unwrap(),
                    &token_program_id,
                ),
                spl_token_2022::instruction::initialize_mint(
                    &token_program_id,
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    decimals,
                )
                .unwrap(),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_account],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        mint_account.pubkey()
    }

    async fn init_new_mint_with_fee(
        &mut self,
        mint_authority: Pubkey,
        token_program_id: Pubkey,
        fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
        transfer_fee_config_authority: Option<&Pubkey>,
        withdraw_withheld_authority: Option<&Pubkey>,
    ) -> Pubkey {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let mint_account = Keypair::new();
        let rent = self.banks_client.get_rent().await.unwrap();
        let space =
            ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferFeeConfig])
                .unwrap();

        let transaction = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &self.payer.pubkey(),
                    &mint_account.pubkey(),
                    rent.minimum_balance(space),
                    space.try_into().unwrap(),
                    &token_program_id,
                ),
                spl_token_2022::extension::transfer_fee::instruction::initialize_transfer_fee_config(
                    &token_program_id,
                    &mint_account.pubkey(),
                    transfer_fee_config_authority,
                    withdraw_withheld_authority,
                    fee_basis_points,
                    maximum_fee
                ).unwrap(),
                spl_token_2022::instruction::initialize_mint(
                    &token_program_id,
                    &mint_account.pubkey(),
                    &mint_authority,
                    None,
                    decimals
                )
                .unwrap(),
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_account],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();

        mint_account.pubkey()
    }

    async fn mint_tokens_to(
        &mut self,
        mint: Pubkey,
        to: Pubkey,
        mint_authority: Keypair,
        amount: u64,
        token_program_id: Pubkey,
    ) {
        let recent_blockhash = self.banks_client.get_latest_blockhash().await.unwrap();
        let ix = spl_token_2022::instruction::mint_to(
            &token_program_id,
            &mint,
            &to,
            &mint_authority.pubkey(),
            &[&mint_authority.pubkey()],
            amount,
        )
        .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &mint_authority],
            recent_blockhash,
        );
        self.banks_client
            .process_transaction(transaction)
            .await
            .unwrap();
    }
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
