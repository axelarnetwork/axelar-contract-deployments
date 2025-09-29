//! Program state processor.

use borsh::BorshDeserialize;
use event_cpi_macros::event_cpi_handler;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;

use crate::check_program_account;
use crate::instructions::GatewayInstruction;

mod approve_message;
mod call_contract;
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

#[allow(clippy::multiple_inherent_impl)]
/// Program state handler.
pub struct Processor;

impl Processor {
    /// Main entry point for processing Gateway program instructions.
    ///
    /// Deserializes the instruction data and delegates to the appropriate instruction-specific
    /// processor method.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Instruction deserialization fails.
    /// * Program ID validation fails.
    /// * Downstream processing fails.
    // Reason for `clippy::too_many_lines`:
    // This is intentionally a long function since it serves as the main program entry point,
    // so the lint's warning is expected here - the function handles all core routing logic
    // through a single `match` statement.
    #[allow(clippy::too_many_lines)]
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        check_program_account(*program_id)?;

        event_cpi_handler!(input);

        let instruction = GatewayInstruction::try_from_slice(input)?;

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
                signing_pda_bump,
            } => {
                msg!("Instruction: Call Contract");
                Self::process_call_contract(
                    program_id,
                    accounts,
                    &destination_chain,
                    &destination_contract_address,
                    payload,
                    signing_pda_bump,
                )
            }
            GatewayInstruction::InitializeConfig(init_config) => {
                msg!("Instruction: Initialize Config");
                Self::process_initialize_config(program_id, accounts, &init_config)
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
                    &verifier_info,
                )
            }
            GatewayInstruction::ValidateMessage { message } => {
                msg!("Instruction: Validate Message");
                Self::process_validate_message(program_id, accounts, &message)
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
