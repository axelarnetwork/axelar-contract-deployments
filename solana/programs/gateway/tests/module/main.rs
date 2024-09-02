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
use std::str::FromStr;

use axelar_message_primitives::{DataPayload, EncodingScheme, U256};
use axelar_rkyv_encoding::hash_payload;
use axelar_rkyv_encoding::types::{
    ExecuteData, Message, Payload, Proof, PublicKey, WeightedSigner,
};
use gmp_gateway::commands::OwnedCommand;
use gmp_gateway::events::{EventContainer, GatewayEvent};
use gmp_gateway::hasher_impl;
use gmp_gateway::state::execute_data::{ApproveMessagesVariant, RotateSignersVariant};
use gmp_gateway::state::{GatewayApprovedCommand, GatewayExecuteData};
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

fn get_approve_messages_gateway_events_from_execute_data(
    execute_data: &GatewayExecuteData<ApproveMessagesVariant>,
) -> Vec<EventContainer> {
    execute_data
        .data
        .iter()
        .map(|x| {
            let event = GatewayEvent::MessageApproved(gmp_gateway::events::MessageApproved {
                command_id: x.cc_id().command_id(hasher_impl()),
                source_chain: x.cc_id().chain().to_owned().into_bytes(),
                message_id: x.cc_id().id().to_owned().into_bytes(),
                source_address: x.source_address().into(),
                destination_address: Pubkey::from_str(x.destination_address())
                    .unwrap()
                    .to_bytes(),
                payload_hash: *x.payload_hash(),
            });
            let vec = event.encode();
            EventContainer::new(vec.to_vec()).unwrap()
        })
        .collect()
}

fn get_rotate_signers_gateway_events_from_execute_data(
    execute_data: GatewayExecuteData<RotateSignersVariant>,
    gateway_root_pda: &Pubkey,
    expected_epoch: U256,
) -> EventContainer {
    let event = GatewayEvent::SignersRotated(gmp_gateway::events::RotateSignersEvent {
        new_epoch: expected_epoch,
        new_signers_hash: execute_data.data.hash(hasher_impl()),
        execute_data_pda: gmp_gateway::get_execute_data_pda(
            gateway_root_pda,
            &execute_data.hash_decoded_contents(),
        )
        .0
        .to_bytes(),
    });
    let vec = event.encode();
    EventContainer::new(vec.to_vec()).unwrap()
}

fn get_gateway_events(
    tx: &solana_program_test::BanksTransactionResultWithMetadata,
) -> Vec<EventContainer> {
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
