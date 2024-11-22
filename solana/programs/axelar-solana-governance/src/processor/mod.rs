//! Program instructions processor.
//!
//! The are 2 levels of instructions:
//!
//! 1. Governance GMP Instructions: These are the main instructions that come
//!    from the Axelar governance infrastructure.
//! 2. Native program instructions: These are the instructions that are executed
//!    by other Solana addresses.

use axelar_rkyv_encoding::hasher::solana::SolanaKeccak256Hasher;
use axelar_rkyv_encoding::hasher::{AxelarRkyv256Hasher, Hash256};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instructions::GovernanceInstruction;
use crate::{check_program_account, seed_prefixes};

mod execute_operator_proposal;
mod execute_proposal;
mod gmp;
mod init_config;
mod transfer_operatorship;
mod withdraw_tokens;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    ///
    /// # Errors
    ///
    /// A `ProgramError` containing the error that occurred is returned. Log
    /// messages are also generated with more detailed information.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        instruction_data: &[u8],
    ) -> ProgramResult {
        check_program_account(*program_id)?;

        let governance_instruction =
            GovernanceInstruction::from_bytes(instruction_data).map_err(|err| {
                msg!("Could not decode program input data: {}", err);
                ProgramError::InvalidArgument
            })?;

        match governance_instruction {
            GovernanceInstruction::InitializeConfig(governance_config) => {
                init_config::process(program_id, accounts, governance_config)
            }
            // GMP instructions
            GovernanceInstruction::GovernanceGmpPayload { payload, metadata } => {
                gmp::process(program_id, accounts, &payload, &metadata)
            }
            GovernanceInstruction::ExecuteProposal(execute_proposal) => {
                execute_proposal::process(program_id, accounts, &execute_proposal)
            }
            GovernanceInstruction::ExecuteOperatorProposal(execute_proposal_data) => {
                execute_operator_proposal::process(program_id, accounts, &execute_proposal_data)
            }

            GovernanceInstruction::WithdrawTokens { amount } => {
                withdraw_tokens::process(program_id, accounts, amount)
            }
            GovernanceInstruction::TransferOperatorship { new_operator } => {
                transfer_operatorship::process(program_id, accounts, new_operator)
            }
        }
    }
}

fn hash(data: &[u8]) -> Hash256 {
    let mut hasher = SolanaKeccak256Hasher::default();
    hasher.hash(data);
    hasher.result()
}

/// Ensure that the governance PDA has been derived correctly
///
/// # Errors
///
/// This function will return an error if the provided pubkey does not match the
/// expected pubkey.
#[inline]
pub fn ensure_valid_governance_root_pda(
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    #[allow(clippy::expect_used)]
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::GOVERNANCE_CONFIG, &[bump]], &crate::ID)
            .map_err(|err| {
                msg!("Invalid bump for the root config pda: {}", err);
                ProgramError::InvalidAccountData
            })?;

    if &derived_pubkey != expected_pubkey {
        msg!("Invalid config/root pda");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
