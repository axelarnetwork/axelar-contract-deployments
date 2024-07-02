// We allow this so `cargo check --tests` doesn't emit warnings for unused crate
// dependencies in test binaries.
#![allow(unused_crate_dependencies)]

mod approve_messages;
mod initialize_command;
mod initialize_config;
mod initialize_execute_data;
mod rotate_signers;
mod transfer_operatorship;

use std::collections::BTreeMap;

use axelar_message_primitives::{DataPayload, EncodingScheme};
use axelar_rkyv_encoding::hash_payload;
use axelar_rkyv_encoding::types::{
    ExecuteData, Message, Payload, Proof, VerifierSet, WeightedSignature,
};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::state::GatewayApprovedCommand;
use solana_program_test::tokio::fs;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use test_fixtures::axelar_message::{custom_message, new_signer_set};
use test_fixtures::execute_data::TestSigner;
use test_fixtures::test_setup::TestFixture;
use test_fixtures::test_signer::create_signer_with_weight;

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
        .map(|weight| create_signer_with_weight(*weight))
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

pub fn example_signer_set(threshold: u128, created_at_block: u64) -> VerifierSet {
    let new_signers = vec![create_signer_with_weight(threshold)];
    new_signer_set(&new_signers, created_at_block, threshold)
}

pub fn gateway_approved_command_ixs(
    commands: &[OwnedCommand],
    gateway_root_pda: Pubkey,
    fixture: &TestFixture,
) -> Vec<(Pubkey, solana_sdk::instruction::Instruction)> {
    let ixs = commands
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

fn get_gateway_events_from_execute_data(commands: &[OwnedCommand]) -> Vec<GatewayEvent<'static>> {
    commands
        .iter()
        .cloned()
        .map(gmp_gateway::events::GatewayEvent::try_from)
        .collect::<Result<Vec<_>, _>>()
        .expect("failed to parse events from execute_data")
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

pub fn create_signer_set(weights: &[u128], threshold: u128) -> (VerifierSet, Vec<TestSigner>) {
    let new_signers = weights
        .iter()
        .copied()
        .map(create_signer_with_weight)
        .collect::<Vec<_>>();
    let created_at = unix_seconds();
    let new_signer_set = new_signer_set(&new_signers, created_at, threshold);
    (new_signer_set, new_signers)
}

/// FIXME: I'm not sure if the old code did the same, but it turns out that the
/// 'signers_for_submission' field is never used.
#[allow(unused_variables)]
pub fn prepare_questionable_execute_data(
    payload_for_signing: &Payload,
    payload_for_submission: &Payload,
    signers_for_signing: &[TestSigner],
    signers_for_submission: &[TestSigner],
    threshold: u128,
    domain_separator: &[u8; 32],
) -> Vec<u8> {
    let other_execute_data = create_execute_data(
        payload_for_signing,
        signers_for_signing,
        threshold,
        domain_separator,
    );
    let other_proof = other_execute_data.proof().clone();

    let execute_data_for_submission = ExecuteData::new(payload_for_submission.clone(), other_proof);

    execute_data_for_submission
        .to_bytes::<0>()
        .expect("failed to serialize 'ExecuteData' struct")
}

fn create_execute_data(
    payload: &Payload,
    signers: &[TestSigner],
    threshold: u128,
    domain_separator: &[u8; 32],
) -> ExecuteData {
    let nonce = unix_seconds();
    let verifier_set: VerifierSet = create_verifier_set_with_nonce(signers, nonce, threshold);

    let payload_hash = hash_payload(domain_separator, &verifier_set, payload);

    let signing_keys: BTreeMap<_, _> = signers
        .iter()
        .map(|signer| (signer.public_key, &signer.secret_key))
        .collect();

    let weighted_signatures: Vec<_> = verifier_set
        .signers()
        .iter()
        .map(|(pubkey, weight)| {
            let signing_key = signing_keys.get(pubkey).unwrap();
            let signature = signing_key.sign(&payload_hash);
            WeightedSignature::new(*pubkey, signature, *weight)
        })
        .collect();
    let proof = Proof::new(weighted_signatures, *verifier_set.threshold(), nonce);

    ExecuteData::new(payload.clone(), proof)
}

pub fn create_verifier_set_with_nonce(
    signers: &[TestSigner],
    nonce: u64,
    threshold: u128,
) -> VerifierSet {
    let signers: BTreeMap<_, _> = signers
        .iter()
        .map(|signer| (signer.public_key, signer.weight))
        .collect();
    VerifierSet::new(nonce, signers, threshold.into())
}

/// Works as a PRNG for filling in nonces
fn unix_seconds() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn make_message() -> Message {
    custom_message(Pubkey::new_unique(), example_payload())
}

pub fn make_messages(num_messages: usize) -> Vec<Message> {
    (0..num_messages).map(|_| make_message()).collect()
}

pub fn payload_and_commands(messages: &[Message]) -> (Payload, Vec<OwnedCommand>) {
    let payload = Payload::Messages(messages.to_vec());
    let commands = messages
        .iter()
        .cloned()
        .map(OwnedCommand::ApproveMessage)
        .collect();
    (payload, commands)
}

pub fn make_payload_and_commands(num_messages: usize) -> (Payload, Vec<OwnedCommand>) {
    let messages = make_messages(num_messages);
    payload_and_commands(&messages)
}

pub fn make_signers(weights: &[u128]) -> Vec<TestSigner> {
    weights
        .iter()
        .copied()
        .map(create_signer_with_weight)
        .collect::<Vec<_>>()
}
