#![deny(missing_docs)]

//! Utility functions for on-chain integration with the Axelar Gatewey on Solana

use std::borrow::Cow;
use std::ops::Deref;

pub use axelar_message_primitives;
use axelar_message_primitives::{
    AxelarMessageParams, CommandId, DataPayload, DestinationProgramId, SourceAddress, SourceChain,
};
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
/// 2. `gateway_root_pda` - Gateway root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the `DataPayload` constructor
pub fn validate_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    data: &AxelarExecutablePayload,
) -> ProgramResult {
    msg!("Validating contract call");
    let command_id = data.command_id();
    let source_chain = data.source_chain();
    let source_address = data.source_address();

    let (relayer_prepended_accs, origin_chain_provided_accs) = accounts.split_at(4);
    let account_info_iter = &mut relayer_prepended_accs.iter();
    let gateway_approved_message_pda = next_account_info(account_info_iter)?;
    let signing_pda = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    let axelar_payload = DataPayload::new(
        data.payload_without_accounts.as_slice(),
        origin_chain_provided_accs,
    );

    let destination_program = DestinationProgramId(*program_id);
    let (signing_pda_derived, signing_pda_derived_bump) =
        destination_program.signing_pda(&command_id);
    if signing_pda.key != &signing_pda_derived {
        return Err(ProgramError::InvalidAccountData);
    }

    let command_id_slice = &command_id.0.deref().clone();
    invoke_signed(
        &gateway::instructions::validate_contract_call(
            gateway_approved_message_pda.key,
            gateway_root_pda.key,
            signing_pda.key,
            AxelarMessageParams {
                command_id,
                source_chain,
                source_address,
                destination_program,
                payload_hash: axelar_payload.hash(),
            },
        )?,
        &[
            gateway_approved_message_pda.clone(),
            gateway_root_pda.clone(),
            signing_pda.clone(),
        ],
        &[&[command_id_slice, &[signing_pda_derived_bump]]],
    )?;

    Ok(())
}

// TOOD: Optimisation - try using bytemuck crate
/// This is the payload that the `execute` processor on the destinatoin program
/// must expect
#[derive(Debug, PartialEq, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
#[repr(C)]
pub struct AxelarExecutablePayload {
    /// The command_id which is the unique identifier for the Axelar command
    ///
    /// The Axelar Message CCID, truncated to 32 bytes during proof
    /// generation.
    pub command_id: [u8; 32],
    /// The payload *without* the prefixed accounts
    ///
    /// This needs to be done by the relayer before calling the destination
    /// program
    pub payload_without_accounts: Vec<u8>,
    /// The source chain of the command
    pub source_chain: String,
    /// The source address of the command
    pub source_address: Vec<u8>,
}

// TOOD: Optimisation - try using bytemuck crate
/// This is the wrapper instruction that the destination program should expect
/// as the incoming &[u8]
#[derive(Debug, PartialEq, Clone, borsh::BorshSerialize, borsh::BorshDeserialize)]
pub enum AxelarCallableInstruction<T> {
    /// The payload is coming from the Axelar Gateway (submitted by the relayer)
    AxelarExecute(AxelarExecutablePayload),
    /// The payload is coming from the user
    Custom(T),
}

impl AxelarExecutablePayload {
    /// Get the command_id
    pub fn command_id(&self) -> CommandId {
        CommandId(Cow::Borrowed(&self.command_id))
    }

    /// Get the source chain
    pub fn source_chain(&self) -> SourceChain {
        SourceChain(Cow::Borrowed(&self.source_chain))
    }

    /// Get the source address
    pub fn source_address(&self) -> SourceAddress {
        SourceAddress(&self.source_address)
    }
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
/// 2. `gateway_root_pda` - Gateway root PDA
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N... - The accounts provided in the `axelar_message_payload`
pub fn construct_axelar_executable_ix<'a>(
    incoming_message: impl Into<AxelarMessageParams<'a>>,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: Vec<u8>,
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_approved_message_pda: Pubkey,
    // The PDA for the gateway root, this *must* be initialized beforehand
    gateway_root_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let incoming_message = incoming_message.into();
    let payload = DataPayload::decode(axelar_message_payload.as_slice());
    if payload.hash() != incoming_message.payload_hash {
        return Err(ProgramError::InvalidInstructionData);
    }

    let passed_in_accounts = payload.account_meta();
    let payload_without_accounts = payload.payload_without_accounts().to_vec();

    let (gateway_approved_message_signing_pda, _) = incoming_message
        .destination_program
        .signing_pda(&incoming_message.command_id);

    let payload = AxelarExecutablePayload {
        command_id: *incoming_message.command_id.0,
        payload_without_accounts,
        source_chain: incoming_message.source_chain.0.to_string(),
        source_address: incoming_message.source_address.0.to_vec(),
    };

    let payload = AxelarCallableInstruction::<()>::AxelarExecute(payload);

    let mut accounts = vec![
        // The expected accounts for the `ValidateContractCall` ix
        AccountMeta::new(gateway_approved_message_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
    ];
    accounts.append(&mut passed_in_accounts.to_vec());

    Ok(Instruction {
        program_id: incoming_message.destination_program.0,
        accounts,
        data: borsh::to_vec(&payload)?,
    })
}
