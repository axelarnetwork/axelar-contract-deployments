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
    ExecuteData, Message, Payload, Proof, PublicKey, WeightedSigner,
};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::hasher_impl;
use gmp_gateway::state::GatewayApprovedCommand;
use solana_program_test::{processor, ProgramTest};
use solana_sdk::instruction::AccountMeta;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::Signer;
use test_fixtures::axelar_message::custom_message;
use test_fixtures::test_setup::{SigningVerifierSet, TestFixture};

pub fn program_test() -> ProgramTest {
    ProgramTest::new(
        "gmp_gateway",
        gmp_gateway::id(),
        processor!(gmp_gateway::processor::Processor::process_instruction),
    )
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
    signers_for_signing: &SigningVerifierSet,
    signers_for_submission: &SigningVerifierSet,
    domain_separator: &[u8; 32],
) -> Vec<u8> {
    let payload_hash_for_signing = hash_payload(
        domain_separator,
        &signers_for_submission.verifier_set(),
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
        *signers_for_submission.verifier_set().quorum(),
        signers_for_submission.verifier_set().created_at(),
    );
    let execute_data_for_submission = ExecuteData::new(payload_for_submission.clone(), proof);

    execute_data_for_submission
        .to_bytes::<0>()
        .expect("failed to serialize 'ExecuteData' struct")
}

fn create_signer_array(
    signers: &SigningVerifierSet,
    non_signers: &SigningVerifierSet,
    payload_hash: [u8; 32],
) -> BTreeMap<PublicKey, WeightedSigner> {
    let weighted_signatures = signers
        .signers
        .iter()
        .map(|signer| {
            let signature = signer.secret_key.sign(&payload_hash);
            (
                signer.public_key,
                WeightedSigner::new(Some(signature), signer.weight),
            )
        })
        .chain({
            non_signers.signers.iter().map(|non_signer| {
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

pub fn make_message() -> Message {
    custom_message(Pubkey::new_unique(), &example_payload())
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
