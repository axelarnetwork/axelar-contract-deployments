// We allow this so `cargo check --tests` doesn't emit warnings for unused crate
// dependencies in test binaries.
#![allow(unused_crate_dependencies)]

mod approve_messages;
mod initialize_command;
mod initialize_config;
mod initialize_execute_data;
// mod rotate_signers;
mod transfer_operatorship;

use std::collections::BTreeMap;

use axelar_message_primitives::{DataPayload, EncodingScheme};
use axelar_rkyv_encoding::hash_payload;
use axelar_rkyv_encoding::types::{
    ExecuteData, Message, Payload, Proof, PublicKey, VerifierSet, WeightedSigner,
};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::hasher_impl;
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
    pub nonce: u64,
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
    let nonce = 42;
    let gateway_root_pda = fixture
        .initialize_gateway_config_account(
            fixture.init_auth_weighted_module_custom_threshold(&signers, quorum.into(), nonce),
            operator.pubkey(),
        )
        .await;

    InitialisedGatewayMetadata {
        nonce,
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

/// This allows creating scenaroius where we can play around with a matrix of
/// constructing invalid values. The usual flow is: we have a [`Payload`] that
/// is signed by a set of signers. This function allows us to:
/// - encode a different signer set inside the execute data than the one that
///   was used to sign the payload
/// - sign a different payload than the one that actually gets encoded in the
///   execute data (thus the hashes would not match)
pub fn prepare_questionable_execute_data(
    payload_for_signing: &Payload,
    payload_for_submission: &Payload,
    signers_for_signing: &[TestSigner],
    signers_for_submission: &[TestSigner],
    threshold: u128,
    domain_separator: &[u8; 32],
    nonce: u64,
) -> Vec<u8> {
    let verifier_set_for_submission =
        create_verifier_set_with_nonce(signers_for_submission, nonce, threshold);
    let payload_hash_for_signing = hash_payload(
        domain_separator,
        &verifier_set_for_submission,
        payload_for_signing,
        hasher_impl(),
    );
    let signatures = create_signer_array(
        signers_for_signing,
        signers_for_submission,
        payload_hash_for_signing,
    );
    let proof = Proof::new(
        signatures,
        *verifier_set_for_submission.threshold(),
        verifier_set_for_submission.created_at(),
    );
    let execute_data_for_submission = ExecuteData::new(payload_for_submission.clone(), proof);

    execute_data_for_submission
        .to_bytes::<0>()
        .expect("failed to serialize 'ExecuteData' struct")
}

fn create_signer_array(
    signers: &[TestSigner],
    non_signers: &[TestSigner],
    payload_hash: [u8; 32],
) -> BTreeMap<PublicKey, WeightedSigner> {
    let weighted_signatures = signers
        .iter()
        .map(|signer| {
            let signature = signer.secret_key.sign(&payload_hash);
            (
                signer.public_key,
                WeightedSigner::new(Some(signature), signer.weight),
            )
        })
        .chain({
            non_signers.iter().map(|non_signer| {
                (
                    non_signer.public_key,
                    WeightedSigner::new(None, non_signer.weight),
                )
            })
        })
        .fold(
            BTreeMap::<PublicKey, WeightedSigner>::new(),
            |mut init, i| {
                if let std::collections::btree_map::Entry::Vacant(e) = init.entry(i.0) {
                    e.insert(i.1);
                }
                init
            },
        );
    weighted_signatures
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
    let payload = Payload::new_messages(messages.to_vec());
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
