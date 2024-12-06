//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use core::borrow::Borrow;
use core::str::FromStr;

use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::get_validate_message_signing_pda;
use axelar_solana_gateway::state::incoming_message::{command_id, IncomingMessageWrapper};
use axelar_solana_gateway::state::BytemuckedPda;
use num_traits::{FromPrimitive, ToPrimitive};
use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Deserialize, Infallible};
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

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 3;

/// Perform CPI call to the Axelar Gateway to ensure that the given message is
/// approved.
///
/// The check will ensure that the provided accounts are indeed the ones that
/// were originated on the source chain.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the `DataPayload` constructor
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_message(
    accounts: &[AccountInfo<'_>],
    data: &ArchivedAxelarExecutablePayload,
) -> ProgramResult {
    let (relayer_prepended_accs, origin_chain_provided_accs) =
        accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);

    let signing_pda_bump = {
        // scope to release the account after reading the data we want
        let accounts_iter = &mut relayer_prepended_accs.iter();
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessageWrapper::read(&incoming_message_data)?;
        incoming_message.signing_pda_bump
    };

    let axelar_payload = AxelarMessagePayload::new(
        data.payload_without_accounts.as_slice(),
        origin_chain_provided_accs,
        EncodingScheme::from_u8(data.encoding_scheme).ok_or(ProgramError::InvalidArgument)?,
    );

    validate_message_internal(
        accounts,
        &data
            .message
            .deserialize(&mut Infallible)
            .map_err(|_err| ProgramError::InvalidArgument)?,
        axelar_payload.hash()?.0.borrow(),
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
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the inner instruction (part of the payload).
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_with_gmp_metadata(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload: &[u8],
) -> ProgramResult {
    let signing_pda_bump = {
        // scope to release the account after reading the data we want
        let accounts_iter = &mut accounts.iter();
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessageWrapper::read(&incoming_message_data)?;
        incoming_message.signing_pda_bump
    };

    let payload_hash = solana_program::keccak::hash(payload).to_bytes();
    validate_message_internal(accounts, message, &payload_hash, signing_pda_bump)
}

fn validate_message_internal(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload_hash: &[u8; 32],
    signing_pda_derived_bump: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let gateway_incoming_message = next_account_info(account_info_iter)?;
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
/// 1. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 2. `gateway_root_pda` - Gateway Root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N... - The accounts provided in the `axelar_message_payload`
///
/// # Errors
/// - if the destination address is not a vald base58 encoded ed25519 pubkey
/// - if the `axelar_message_payload` could not be decoded
/// - if we cannot encode the `AxelarExecutablePayload`
pub fn construct_axelar_executable_ix(
    message: Message,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: &[u8],
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_incoming_message: Pubkey,
) -> Result<Instruction, ProgramError> {
    let payload = AxelarMessagePayload::decode(axelar_message_payload)?;

    let mut passed_in_accounts = payload.account_meta();
    let payload_without_accounts = payload.payload_without_accounts().to_vec();
    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_er| ProgramError::InvalidAccountData)?;

    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let (signing_pda, _) = get_validate_message_signing_pda(destination_address, command_id);

    let payload = AxelarExecutablePayload {
        payload_without_accounts,
        message,
        encoding_scheme: payload
            .encoding_scheme()
            .to_u8()
            .ok_or(ProgramError::InvalidArgument)?,
    };
    let encoded_payload = rkyv::to_bytes::<_, 0>(&payload)
        .map_err(|_err| ProgramError::InvalidArgument)?
        .to_vec();

    let mut payload = AXELAR_EXECUTE.to_vec();
    payload.extend_from_slice(&encoded_payload);

    let mut accounts = vec![
        // The expected accounts for the `ValidateMessage` ix
        AccountMeta::new(gateway_incoming_message, false),
        AccountMeta::new_readonly(signing_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
    ];
    accounts.append(&mut passed_in_accounts);

    Ok(Instruction {
        program_id: destination_address,
        accounts,
        data: payload,
    })
}

/// This is the payload that the `execute` processor on the destinatoin program
/// must expect
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
#[repr(C)]
pub struct AxelarExecutablePayload {
    /// The bytes for an `axelar-rkyv-encoding::Message` value.
    pub message: Message,

    /// The payload *without* the prefixed accounts
    ///
    /// This needs to be done by the relayer before calling the destination
    /// program
    pub payload_without_accounts: Vec<u8>,

    /// The encoding scheme used to encode this payload.
    pub encoding_scheme: u8,
}

/// Axelar executable command prefix
pub const AXELAR_EXECUTE: &[u8; 16] = b"axelar-execute__";

/// Utility trait to extract the `AxelarExecutablePayload`
pub trait MaybeAxelarPayload {
    /// Try to extract the `AxearlExecutablePayload` from the given payload
    fn try_get_axelar_executable_payload(
        &self,
    ) -> Option<Result<&ArchivedAxelarExecutablePayload, ProgramError>>;
}

impl MaybeAxelarPayload for &[u8] {
    fn try_get_axelar_executable_payload(
        &self,
    ) -> Option<Result<&ArchivedAxelarExecutablePayload, ProgramError>> {
        let first_16_bytes = self.get(0..AXELAR_EXECUTE.len())?;
        if first_16_bytes != AXELAR_EXECUTE {
            return None;
        }
        let all_other_bytes = self.get(AXELAR_EXECUTE.len()..)?;
        let result = rkyv::check_archived_root::<AxelarExecutablePayload>(all_other_bytes)
            .map_err(|_err| ProgramError::InvalidInstructionData);
        Some(result)
    }
}
