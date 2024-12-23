//! Program state processor.

use borsh::BorshDeserialize;
pub use event_utils::EventParseError;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::check_program_account;
use crate::instructions::GatewayInstruction;

mod approve_message;
mod call_contract;
mod call_contract_offchain_data;
mod close_message_payload;
mod commit_message_payload;
mod initialize_config;
mod initialize_message_payload;
mod initialize_payload_verification_session;
mod rotate_signers;
mod transfer_operatorship;
mod validate_message;
mod verify_signature;
mod write_message_payload;

pub use call_contract::CallContractEvent;
pub use call_contract_offchain_data::CallContractOffchainDataEvent;
pub use rotate_signers::VerifierSetRotated;
pub use transfer_operatorship::OperatorshipTransferredEvent;
pub use validate_message::MessageEvent;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = GatewayInstruction::try_from_slice(input)?;
        check_program_account(*program_id)?;

        match instruction {
            GatewayInstruction::ApproveMessage {
                message,
                payload_merkle_root,
            } => {
                msg!("Instruction: Approve Messages");
                Self::process_approve_message(program_id, accounts, message, payload_merkle_root)
            }
            GatewayInstruction::RotateSigners {
                new_verifier_set_merkle_root,
            } => {
                msg!("Instruction: Rotate Signers");
                Self::process_rotate_verifier_set(
                    program_id,
                    accounts,
                    new_verifier_set_merkle_root,
                )
            }
            GatewayInstruction::CallContract {
                destination_chain,
                destination_contract_address,
                payload,
            } => {
                msg!("Instruction: Call Contract");
                Self::process_call_contract(
                    program_id,
                    accounts,
                    destination_chain,
                    destination_contract_address,
                    payload,
                )
            }
            GatewayInstruction::CallContractOffchainData {
                destination_chain,
                destination_contract_address,
                payload_hash,
            } => {
                msg!("Instruction: Call Contract Offchain Data");
                Self::process_call_contract_offchain_data(
                    program_id,
                    accounts,
                    destination_chain,
                    destination_contract_address,
                    payload_hash,
                )
            }
            GatewayInstruction::InitializeConfig(init_config) => {
                msg!("Instruction: Initialize Config");
                Self::process_initialize_config(program_id, accounts, init_config)
            }

            GatewayInstruction::InitializePayloadVerificationSession {
                payload_merkle_root,
            } => {
                msg!("Instruction: Initialize Verification Session");
                Self::process_initialize_payload_verification_session(
                    program_id,
                    accounts,
                    payload_merkle_root,
                )
            }

            GatewayInstruction::VerifySignature {
                payload_merkle_root,
                verifier_info,
            } => {
                msg!("Instruction: Verify Signature");
                Self::process_verify_signature(
                    program_id,
                    accounts,
                    payload_merkle_root,
                    verifier_info,
                )
            }
            GatewayInstruction::ValidateMessage { message } => {
                msg!("Instruction: Validate Message");
                Self::process_validate_message(program_id, accounts, message)
            }
            GatewayInstruction::InitializeMessagePayload {
                buffer_size,
                command_id,
            } => {
                msg!("Instruction: Initialize Message Payload");
                Self::process_initialize_message_payload(
                    program_id,
                    accounts,
                    buffer_size,
                    command_id,
                )
            }
            GatewayInstruction::WriteMessagePayload {
                offset,
                bytes,
                command_id,
            } => {
                msg!("Instruction: Write Message Payload");
                Self::process_write_message_payload(
                    program_id, accounts, offset, &bytes, command_id,
                )
            }
            GatewayInstruction::CloseMessagePayload { command_id } => {
                msg!("Instruction: Close Message Payload");
                Self::process_close_message_payload(program_id, accounts, command_id)
            }
            GatewayInstruction::CommitMessagePayload { command_id } => {
                msg!("Instruction: Commit Message Payload");
                Self::process_commit_message_payload(program_id, accounts, command_id)
            }
            GatewayInstruction::TransferOperatorship => {
                msg!("Instruction: Transfer Operatorship");
                Self::process_transfer_operatorship(program_id, accounts)
            }
        }
    }
}

/// Represents the various events emitted by the Gateway.
///
/// The `GatewayEvent` enum encapsulates all possible events that can be emitted by the Gateway.
/// Each variant corresponds to a specific event type and contains the relevant data associated with that event.
///
/// These events are crucial for monitoring the state and actions within the Gateway, such as contract calls,
/// verifier set rotations, operatorship transfers, and message approvals and executions.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GatewayEvent {
    /// Represents a `CallContract` event.
    ///
    /// This event is emitted when a contract call is initiated to an external chain.
    CallContract(CallContractEvent),

    /// Represents a `CallContractOffchainData` event.
    ///
    /// This event is emitted when a contract call is initiated to an external chain with call data
    /// being passed offchain.
    CallContractOffchainData(CallContractOffchainDataEvent),

    /// Represents a `VerifierSetRotated` event.
    VerifierSetRotated(VerifierSetRotated),

    /// Represents an `OperatorshipTransferred` event.
    ///
    /// This event is emitted when the operatorship is transferred to a new operator.
    /// It includes the public key of the new operator.
    OperatorshipTransferred(OperatorshipTransferredEvent),

    /// Represents a `MessageApproved` event.
    ///
    /// This event is emitted when a message is approved for execution by the Gateway.
    MessageApproved(MessageEvent),

    /// Represents a `MessageExecuted` event.
    ///
    /// This event is emitted when a message has been received & execution has begun on the destination contract.
    MessageExecuted(MessageEvent),
}

pub(crate) mod event_utils {

    /// Errors that may occur while parsing a `MessageEvent`.
    #[derive(Debug, thiserror::Error)]
    pub enum EventParseError {
        /// Occurs when a required field is missing in the event data.
        #[error("Missing data: {0}")]
        MissingData(&'static str),

        /// Occurs when the length of a field does not match the expected length.
        #[error("Invalid length for {field}: expected {expected}, got {actual}")]
        InvalidLength {
            /// the field that we're trying to parse
            field: &'static str,
            /// the desired length
            expected: usize,
            /// the actual length
            actual: usize,
        },

        /// Occurs when a field contains invalid UTF-8 data.
        #[error("Invalid UTF-8 in {field}: {source}")]
        InvalidUtf8 {
            /// the field we're trying to parse
            field: &'static str,
            /// underlying error
            #[source]
            source: std::string::FromUtf8Error,
        },

        /// Generic error for any other parsing issues.
        #[error("Other error: {0}")]
        Other(&'static str),
    }

    pub(crate) fn read_array<const N: usize>(
        field: &'static str,
        data: &[u8],
    ) -> Result<[u8; N], EventParseError> {
        if data.len() != N {
            return Err(EventParseError::InvalidLength {
                field,
                expected: N,
                actual: data.len(),
            });
        }
        let array = data
            .try_into()
            .map_err(|_| EventParseError::InvalidLength {
                field,
                expected: N,
                actual: data.len(),
            })?;
        Ok(array)
    }

    pub(crate) fn read_string(
        field: &'static str,
        data: Vec<u8>,
    ) -> Result<String, EventParseError> {
        String::from_utf8(data).map_err(|e| EventParseError::InvalidUtf8 { field, source: e })
    }
}
