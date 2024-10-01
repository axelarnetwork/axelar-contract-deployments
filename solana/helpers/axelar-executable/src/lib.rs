#![deny(missing_docs)]

//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use std::borrow::Borrow;
use std::str::FromStr;

pub use axelar_message_primitives;
use axelar_message_primitives::{DataPayload, DestinationProgramId, EncodingScheme};
use axelar_rkyv_encoding::types::{ArchivedMessage, GmpMetadata, Message};
use gateway::commands::MessageWrapper;
use gateway::hasher_impl;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 4;

/// Perform CPI call to the Axelar Gateway to ensure that the given command is
/// approved
///
/// Expected accounts:
/// 0. `gateway_approved_message_pda` - GatewayApprovedMessage PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the `DataPayload` constructor
pub fn validate_message(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    data: &AxelarExecutablePayload,
) -> ProgramResult {
    msg!("Validating contract call");

    let (_relayer_prepended_accs, origin_chain_provided_accs) =
        accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);

    let axelar_payload = DataPayload::new(
        data.payload_without_accounts.as_slice(),
        origin_chain_provided_accs,
        data.encoding_scheme,
    );

    validate_message_internal(
        program_id,
        accounts,
        &data.message,
        axelar_payload.hash()?.0.borrow(),
    )
}

/// Perform CPI call to the Axelar Gateway to ensure that the given command
/// (containing an ITS message) is approved
///
/// Expected accounts:
/// 0. `gateway_approved_message_pda` - GatewayApprovedMessage PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the inner instruction (part of the payload).
pub fn validate_with_gmp_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    gmp_metadata: GmpMetadata,
    payload: &[u8],
) -> ProgramResult {
    let payload_hash = solana_program::keccak::hash(payload).to_bytes();
    let message_wrapper: MessageWrapper = Message::new(
        gmp_metadata.cross_chain_id,
        gmp_metadata.source_address,
        gmp_metadata.destination_chain,
        gmp_metadata.destination_address,
        payload_hash,
    )
    .try_into()?;

    validate_message_internal(program_id, accounts, &message_wrapper, &payload_hash)
}

fn validate_message_internal(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    message: &MessageWrapper,
    payload_hash: &[u8; 32],
) -> ProgramResult {
    msg!("Validating contract call");

    let account_info_iter = &mut accounts.iter();
    let gateway_approved_message_pda = next_account_info(account_info_iter)?;
    let signing_pda = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    // Build the actual Message we are going to use
    let archived_message: &ArchivedMessage = message.try_into()?;
    let command_id = archived_message.cc_id().command_id(hasher_impl());

    // Check: Original message's payload_hash is equivalent to provided payload's
    // hash
    if archived_message.payload_hash() != payload_hash {
        msg!("Invalid payload hash");
        return Err(ProgramError::InvalidInstructionData);
    }

    let destination_program = DestinationProgramId(*program_id);
    let (signing_pda_derived, signing_pda_derived_bump) =
        destination_program.signing_pda(&command_id);
    if signing_pda.key != &signing_pda_derived {
        msg!("Invalid signing PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    invoke_signed(
        &gateway::instructions::validate_message(
            gateway_approved_message_pda.key,
            gateway_root_pda.key,
            signing_pda.key,
            message.clone(),
        )?,
        &[
            gateway_approved_message_pda.clone(),
            gateway_root_pda.clone(),
            signing_pda.clone(),
        ],
        &[&[&command_id, &[signing_pda_derived_bump]]],
    )?;

    Ok(())
}

/// # Create a generic `Execute` instruction
///
/// Intended to be used by the relayer when it is about to call the
/// destination program.
///
/// It will prepend the accounts array with these predefined accounts
/// 0. `gateway_approved_message_pda` - GatewayApprovedMessage PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N... - The accounts provided in the `axelar_message_payload`
pub fn construct_axelar_executable_ix(
    incoming_message: Message,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: Vec<u8>,
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_approved_message_pda: Pubkey,
    // The PDA for the gateway root, this *must* be initialized beforehand
    gateway_root_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let payload = DataPayload::decode(axelar_message_payload.as_slice())?;

    // Check: decoded payload_hash and message.payload_hash are the same
    let decoded_payload_hash = payload.hash()?.0;
    if *decoded_payload_hash != *incoming_message.payload_hash() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let passed_in_accounts = payload.account_meta();
    let payload_without_accounts = payload.payload_without_accounts().to_vec();
    let incoming_message_destination_program_pubkey =
        Pubkey::from_str(incoming_message.destination_address())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let destination_program = DestinationProgramId(incoming_message_destination_program_pubkey);

    let (gateway_approved_message_signing_pda, _) =
        destination_program.signing_pda(&incoming_message.cc_id().command_id(hasher_impl()));

    let payload = AxelarCallableInstruction::AxelarExecute(AxelarExecutablePayload {
        payload_without_accounts,
        message: incoming_message.clone().try_into()?,
        encoding_scheme: payload.encoding_scheme(),
    });

    let mut accounts = vec![
        // The expected accounts for the `ValidateMessage` ix
        AccountMeta::new(gateway_approved_message_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
    ];
    accounts.append(&mut passed_in_accounts.to_vec());

    Ok(Instruction {
        program_id: incoming_message_destination_program_pubkey,
        accounts,
        data: borsh::to_vec(&payload)?,
    })
}

/// This is the payload that the `execute` processor on the destinatoin program
/// must expect
#[derive(Debug, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
#[repr(C)]
pub struct AxelarExecutablePayload {
    /// The bytes for an `axelar-rkyv-encoding::Message` value.
    pub message: MessageWrapper,

    /// The payload *without* the prefixed accounts
    ///
    /// This needs to be done by the relayer before calling the destination
    /// program
    pub payload_without_accounts: Vec<u8>,

    /// The encoding scheme used to encode this payload.
    pub encoding_scheme: EncodingScheme,
}

/// This is the wrapper instruction that the destination program should expect
/// as the incoming &[u8]
#[derive(Debug, PartialEq, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub enum AxelarCallableInstruction {
    /// The payload is coming from the Axelar Gateway (submitted by the relayer)
    AxelarExecute(AxelarExecutablePayload),
    /// The payload is coming from the user
    Native(Vec<u8>),
}
