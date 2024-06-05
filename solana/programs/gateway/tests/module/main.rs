mod approve_messages;
mod initialize_command;
mod initialize_config;
mod initialize_execute_data;
mod rotate_signers;
mod transfer_operatorship;

use axelar_message_primitives::{DataPayload, EncodingScheme};
use cosmwasm_std::Uint256;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
use itertools::Either;
use multisig::worker_set::WorkerSet;
use solana_program_test::tokio::fs;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::axelar_message::new_signer_set;
use test_fixtures::execute_data::{
    create_command_batch, create_signer_with_weight, sign_batch, TestSigner,
};
use test_fixtures::test_setup::TestFixture;

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gmp_gateway",
        gmp_gateway::id(),
        processor!(gmp_gateway::processor::Processor::process_instruction),
    )
}

/// Contains metadata information about the initialised Gateway config
pub struct InitialisedGatewayMetadata {
    pub fixture: TestFixture,
    pub quorum: u128,
    pub signers: Vec<TestSigner>,
    pub gateway_root_pda: Pubkey,
    pub operator: Keypair,
    pub upgrade_authority: Keypair,
}

pub async fn setup_initialised_gateway(
    initial_signer_weights: &[u128],
    custom_quorum: Option<u128>,
) -> InitialisedGatewayMetadata {
    // Create a new ProgramTest instance
    let mut fixture = TestFixture::new(ProgramTest::default()).await;
    // Generate a new keypair for the upgrade authority
    let upgrade_authority = Keypair::new();
    let gateway_program_bytecode = fs::read("../../target/deploy/gmp_gateway.so")
        .await
        .unwrap();
    fixture
        .register_upgradeable_program(
            &gateway_program_bytecode,
            &upgrade_authority.pubkey(),
            &gmp_gateway::id(),
        )
        .await;
    let quorum = custom_quorum.unwrap_or_else(|| initial_signer_weights.iter().sum());
    let signers = initial_signer_weights
        .iter()
        .map(|weight| create_signer_with_weight(*weight).unwrap())
        .collect::<Vec<_>>();
    let operator = Keypair::new();
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module_custom_threshold(&signers, quorum.into()),
            operator.pubkey(),
        )
        .await;

    InitialisedGatewayMetadata {
        upgrade_authority,
        fixture,
        quorum,
        signers,
        gateway_root_pda,
        operator,
    }
}

pub fn example_payload() -> DataPayload<'static> {
    let payload = DataPayload::new(
        b"payload-from-other-chain",
        &[
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
            AccountMeta::new_readonly(Pubkey::new_unique(), false),
        ],
        EncodingScheme::Borsh,
    );
    payload
}

pub fn example_signer_set(new_weight: u128, created_at_block: u64) -> WorkerSet {
    let new_signers = vec![create_signer_with_weight(new_weight).unwrap()];
    new_signer_set(
        &new_signers,
        created_at_block,
        Uint256::from_u128(new_weight),
    )
}

pub fn gateway_approved_command_ixs(
    execute_data: GatewayExecuteData,
    gateway_root_pda: Pubkey,
    fixture: &TestFixture,
) -> Vec<(Pubkey, solana_sdk::instruction::Instruction)> {
    let ixs = execute_data
        .command_batch
        .commands
        .iter()
        .map(|command| {
            let (gateway_approved_message_pda, _bump, _seeds) =
                GatewayApprovedCommand::pda(&gateway_root_pda, command);
            let ix = gmp_gateway::instructions::initialize_pending_command(
                &gateway_root_pda,
                &fixture.payer.pubkey(),
                command.clone(),
            )
            .unwrap();
            (gateway_approved_message_pda, ix)
        })
        .collect::<Vec<_>>();
    ixs
}

fn get_gateway_events_from_execute_data(
    commands: &[axelar_message_primitives::command::DecodedCommand],
) -> Vec<GatewayEvent<'static>> {
    commands
        .iter()
        .cloned()
        .map(gmp_gateway::events::GatewayEvent::from)
        .collect::<Vec<_>>()
}

fn get_gateway_events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<GatewayEvent<'static>> {
    tx.metadata
        .as_ref()
        .unwrap()
        .log_messages
        .iter()
        .filter_map(GatewayEvent::parse_log)
        .collect::<Vec<_>>()
}

pub async fn get_approved_command(
    fixture: &mut test_fixtures::test_setup::TestFixture,
    gateway_approved_command_pda: &Pubkey,
) -> GatewayApprovedCommand {
    fixture
        .get_account::<gmp_gateway::state::GatewayApprovedCommand>(
            gateway_approved_command_pda,
            &gmp_gateway::ID,
        )
        .await
}

pub fn create_signer_set(
    weights: &[impl Into<Uint256> + Copy],
    threshold: impl Into<Uint256>,
) -> (multisig::worker_set::WorkerSet, Vec<TestSigner>) {
    let new_signers = weights
        .iter()
        .map(|weight| {
            create_signer_with_weight({
                let weight: Uint256 = (*weight).into();
                weight
            })
            .unwrap()
        })
        .collect::<Vec<_>>();
    let new_signer_set = new_signer_set(&new_signers, 0, threshold.into());
    (new_signer_set, new_signers)
}

pub fn prepare_questionable_execute_data(
    messages_for_signing: &[Either<connection_router::Message, WorkerSet>],
    messages_for_execute_data: &[Either<connection_router::Message, WorkerSet>],
    signers_for_signatures: &[TestSigner],
    signers_in_the_execute_data: &[TestSigner],
    quorum: u128,
    gateway_root_pda: &Pubkey,
) -> (GatewayExecuteData, Vec<u8>) {
    let command_batch_for_signing = create_command_batch(messages_for_signing).unwrap();
    let command_batch_for_execute_data = create_command_batch(messages_for_execute_data).unwrap();
    let signatures = sign_batch(&command_batch_for_signing, signers_for_signatures).unwrap();
    let encoded_message = test_fixtures::execute_data::encode(
        &command_batch_for_execute_data,
        signers_in_the_execute_data.to_vec(),
        signatures,
        quorum,
    )
    .unwrap();
    let execute_data = GatewayExecuteData::new(encoded_message.as_ref(), gateway_root_pda).unwrap();
    (execute_data, encoded_message)
}
