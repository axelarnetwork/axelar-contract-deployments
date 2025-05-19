//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::error::GatewayError;
use axelar_solana_gateway::state::incoming_message::{command_id, IncomingMessage};
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use axelar_solana_gateway::{get_validate_message_signing_pda, BytemuckedPda};
use core::str::FromStr;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod axelar_payload;
pub use axelar_payload::{
    AxelarMessagePayload, AxelarMessagePayloadHash, EncodingScheme, PayloadError, SolanaAccountRepr,
};

/// Axelar executable command prefix
pub const AXELAR_EXECUTE: &[u8; 16] = b"axelar-execute__";

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 4;

/// Perform CPI call to the Axelar Gateway to ensure that the given message is
/// approved.
///
/// The check will ensure that the provided accounts are indeed the ones that
/// were originated on the source chain.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the `DataPayload` constructor
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_message(accounts: &[AccountInfo<'_>], message: &Message) -> ProgramResult {
    let (relayer_prepended_accs, origin_chain_provided_accs) =
        accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
    let accounts_iter = &mut relayer_prepended_accs.iter();

    let incoming_message_payload_hash;
    let signing_pda_bump = {
        // scope to drop the account borrow after reading the data we want
        let incoming_message_pda = next_account_info(accounts_iter)?;

        // Check: Incoming Message account is owned by the Gateway
        if incoming_message_pda.owner != &axelar_solana_gateway::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        incoming_message_payload_hash = incoming_message.payload_hash;
        incoming_message.signing_pda_bump
    };

    // Check: Message Payload account is owned by the Gateway
    let message_payload_account = next_account_info(accounts_iter)?;
    if message_payload_account.owner != &axelar_solana_gateway::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Read the raw payload from the MessagePayload PDA account
    let message_payload_account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**message_payload_account_data).try_into()?;

    // Check: MessagePayload PDA is finalized
    if !message_payload.committed() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check: MessagePayload's payload hash matches IncomingMessage's
    if *message_payload.payload_hash != incoming_message_payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    // Decode the raw payload
    let axelar_payload = AxelarMessagePayload::decode(message_payload.raw_payload)?;

    // Check: parsed accounts matches the original chain provided accounts
    if !axelar_payload
        .solana_accounts()
        .eq(origin_chain_provided_accs)
    {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(
        accounts,
        message,
        message_payload.payload_hash,
        signing_pda_bump,
    )
}

/// Perform CPI call to the Axelar Gateway to ensure that the given command
/// (containing a GMP message) is approved
///
/// This is useful for contracts that have custom legacy implementations by
/// Axelar on other chains, and therefore they cannot provide the accounts in
/// the GMP message. Therefore, the validation of the accounts becomes the
/// responsibility of the destination program.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the inner instruction (part of the payload).
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_with_gmp_metadata(
    accounts: &[AccountInfo<'_>],
    message: &Message,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let signing_pda_bump = {
        // scope to release the account after reading the data we want
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        incoming_message.signing_pda_bump
    };

    // Check: Message Payload account is owned by the Gateway
    let message_payload_account = next_account_info(accounts_iter)?;
    if message_payload_account.owner != &axelar_solana_gateway::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Read the raw payload from the MessagePayload PDA account
    let message_payload_account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**message_payload_account_data).try_into()?;

    // Check: MessagePayload PDA is finalized
    if !message_payload.committed() {
        return Err(ProgramError::InvalidAccountData);
    }

    let axelar_raw_payload = message_payload.raw_payload;
    let payload_hash = solana_program::keccak::hash(axelar_raw_payload).to_bytes();

    if payload_hash != *message_payload.payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(
        accounts,
        message,
        message_payload.payload_hash,
        signing_pda_bump,
    )
}

fn validate_message_internal(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload_hash: &[u8; 32],
    signing_pda_derived_bump: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let gateway_incoming_message = next_account_info(account_info_iter)?;
    let _message_payload_pda = next_account_info(account_info_iter)?; // skip this one, we don't need it
    let signing_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    // Build the actual Message we are going to use
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

    // Check: Original message's payload_hash is equivalent to provided payload's
    // hash
    if &message.payload_hash != payload_hash {
        msg!("Invalid payload hash");
        return Err(ProgramError::InvalidInstructionData);
    }

    invoke_signed(
        &axelar_solana_gateway::instructions::validate_message(
            gateway_incoming_message.key,
            signing_pda.key,
            message.clone(),
        )?,
        &[gateway_incoming_message.clone(), signing_pda.clone()],
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
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_root_pda` - Gateway Root PDA
/// 4. `gateway_program_id` - Gateway Prorgam ID
/// N... - The accounts provided in the `axelar_message_payload`
///
/// # Errors
/// - if the destination address is not a vald base58 encoded ed25519 pubkey
/// - if the `axelar_message_payload` could not be decoded
/// - if we cannot encode the `AxelarExecutablePayload`
pub fn construct_axelar_executable_ix(
    message: &Message,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: &[u8],
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_incoming_message: Pubkey,
    gateway_message_payload: Pubkey,
) -> Result<Instruction, ProgramError> {
    let passed_in_accounts = AxelarMessagePayload::decode(axelar_message_payload)?.account_meta();

    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_er| ProgramError::InvalidAccountData)?;

    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let (signing_pda, _) = get_validate_message_signing_pda(destination_address, command_id);

    let mut accounts = vec![
        // The expected accounts for the `ValidateMessage` ix
        AccountMeta::new(gateway_incoming_message, false),
        AccountMeta::new_readonly(gateway_message_payload, false),
        AccountMeta::new_readonly(signing_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
    ];
    accounts.extend(passed_in_accounts);

    let data = serialize_message(message)?;

    Ok(Instruction {
        program_id: destination_address,
        accounts,
        data,
    })
}

/// We prefix a byte slice with the literal contents of `AXELAR_EXECUTE` followed
/// by the borsh-serialized Message.
///
/// This two-step approach is needed because borsh demonstrated to exaust a Solana
/// program's memory when trying to deserialize the alternative form (Tag, Message)
/// for an absent tag.
fn serialize_message(message: &Message) -> Result<Vec<u8>, ProgramError> {
    // In our tests, randomly generated messages have, in average, 175 bytes, so 256
    // should be sufficient to avoid reallocations.
    let mut buffer = Vec::with_capacity(256);
    buffer.extend_from_slice(AXELAR_EXECUTE);
    borsh::to_writer(&mut buffer, &message)
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))?;
    Ok(buffer)
}

/// Tries to parse input into an Axelar's message.
///
/// # Errors
/// Will return a `ProgramError::BorshIoError` if parsing fails.
#[allow(clippy::indexing_slicing)]
#[must_use]
pub fn parse_axelar_message(input: &[u8]) -> Option<Result<Message, ProgramError>> {
    // This pre-parsing check is required, otherwise borsh will exhaust the available
    // memory trying to find a possibly missing `AXELAR_EXECUTE` prefix.
    if !input.starts_with(AXELAR_EXECUTE) {
        return None;
    }

    // Slicing: we already checked that slice's lower bound above.
    match borsh::from_slice(&input[AXELAR_EXECUTE.len()..])
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))
    {
        Ok(message) => Some(Ok(message)),
        Err(err) => Some(Err(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_solana_gateway_test_fixtures::gateway::random_message;

    #[test]
    fn test_instruction_serialization() {
        let message = random_message();
        let serialized = serialize_message(&message).unwrap();
        let deserialized = parse_axelar_message(&serialized).unwrap().unwrap();
        assert_eq!(message, deserialized);
    }
}
