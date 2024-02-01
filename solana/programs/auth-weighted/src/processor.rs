//! Program state processor

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::AuthWeightedError;
use crate::instruction::AuthWeightedInstruction;
use crate::types::account::state::AuthWeightedStateAccount;
use crate::types::account::validate_proof::ValidateProofAccount;
use crate::types::u256::U256;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        crate::check_program_account(program_id)?;
        let Ok(instruction) = borsh::de::from_slice(input) else {
            return Err(AuthWeightedError::InvalidInstruction)?;
        };

        match instruction {
            AuthWeightedInstruction::ValidateProof => Self::validate_proof(accounts)?,
        };

        Ok(())
    }

    /// This function takes messageHash and proof data and reverts if proof is
    /// invalid
    pub fn validate_proof(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        // Number of recent operator sets to be tracked.
        const OLD_KEY_RETENTION: u8 = 16;
        let accounts = &mut accounts.iter();

        // A payer isn't used here, but to get further accounts we have to get it first.
        let _ = next_account_info(accounts)?;

        // Account with message hash and proof.
        let params = next_account_info(accounts)?;

        // Account with program state.
        let state = next_account_info(accounts)?;

        // Params account data.
        let params_data: &[u8] = &params.data.borrow();
        let params_data: ValidateProofAccount = borsh::de::from_slice(params_data)?;

        // State account data.
        let state_data: AuthWeightedStateAccount = borsh::de::from_slice(*state.data.borrow())?;

        let operators_hash = params_data.proof.get_operators_hash();

        let operators_epoch = state_data
            .epoch_for_operator_hash(&operators_hash)
            .ok_or(AuthWeightedError::EpochForHashNotFound)?;

        let current_epoch = state_data.current_epoch();

        let operator_epoch_is_outdated = current_epoch
            .checked_sub(*operators_epoch)
            .ok_or(ProgramError::ArithmeticOverflow)?
            >= U256::from(OLD_KEY_RETENTION);

        if *operators_epoch == U256::ZERO || operator_epoch_is_outdated {
            return Err(AuthWeightedError::InvalidOperators)?;
        };

        if *operators_epoch != current_epoch {
            return Err(AuthWeightedError::EpochMissmatch)?;
        }

        params_data.validate()?;

        Ok(())
    }
}
