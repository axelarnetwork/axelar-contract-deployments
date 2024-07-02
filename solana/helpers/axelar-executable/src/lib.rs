#![deny(missing_docs)]

//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use std::str::FromStr;

pub use axelar_message_primitives;
use axelar_message_primitives::{
    AxelarCallableInstruction, AxelarExecutablePayload, DataPayload, DestinationProgramId,
};
use axelar_rkyv_encoding::types::{CrossChainId, Message};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

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
    let command_id = &data.command_id;
    let source_address = data.source_address.clone();

    let (relayer_prepended_accs, origin_chain_provided_accs) = accounts.split_at(4);
    let account_info_iter = &mut relayer_prepended_accs.iter();
    let gateway_approved_message_pda = next_account_info(account_info_iter)?;
    let signing_pda = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    let axelar_payload = DataPayload::new(
        data.payload_without_accounts.as_slice(),
        origin_chain_provided_accs,
        data.encoding_scheme,
    );
    let payload_hash = *axelar_payload.hash()?.0;

    // Build the actual Message we are going to use
    let cc_id = CrossChainId::new(command_id.chain.clone(), command_id.id.clone());
    let message = Message::new(
        cc_id,
        source_address,
        "solana".into(), // FIXME: Check if this is the correct value for the destination chain
        program_id.to_string(),
        payload_hash,
    );
    let command_id_slice = message.hash();

    let destination_program = DestinationProgramId(*program_id);
    let (signing_pda_derived, signing_pda_derived_bump) =
        destination_program.signing_pda(&command_id_slice);
    if signing_pda.key != &signing_pda_derived {
        return Err(ProgramError::InvalidAccountData);
    }

    invoke_signed(
        &gateway::instructions::validate_message(
            gateway_approved_message_pda.key,
            gateway_root_pda.key,
            signing_pda.key,
            message,
        )?,
        &[
            gateway_approved_message_pda.clone(),
            gateway_root_pda.clone(),
            signing_pda.clone(),
        ],
        &[&[&command_id_slice, &[signing_pda_derived_bump]]],
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
    if payload.hash()?.0.as_ref() != incoming_message.payload_hash() {
        return Err(ProgramError::InvalidInstructionData);
    }

    let command_id = incoming_message.hash();

    let passed_in_accounts = payload.account_meta();
    let payload_without_accounts = payload.payload_without_accounts().to_vec();
    let incoming_message_destination_program_pubkey =
        Pubkey::from_str(incoming_message.destination_address())
            .map_err(|_| ProgramError::InvalidAccountData)?;
    let destination_program = DestinationProgramId(incoming_message_destination_program_pubkey);

    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    let incoming_message_ccid = incoming_message.cc_id();
    let cross_chain_id = axelar_message_primitives::CrossChainId {
        chain: incoming_message_ccid.chain().to_string(),
        id: incoming_message_ccid.id().to_string(),
    };
    let payload = AxelarExecutablePayload {
        command_id: cross_chain_id,
        payload_without_accounts,
        source_chain: incoming_message_ccid.chain().into(),
        source_address: incoming_message.source_address().to_string(),
        encoding_scheme: payload.encoding_scheme(),
    };
    let payload = AxelarCallableInstruction::<()>::AxelarExecute(payload);

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
