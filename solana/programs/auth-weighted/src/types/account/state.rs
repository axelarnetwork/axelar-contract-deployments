//! Program state accounts.

use std::collections::BTreeMap;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

use crate::error::AuthWeightedError;
use crate::types::u256::U256;

/// [AuthWeightedStateAccount]; where program keeps operators and epoch.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AuthWeightedStateAccount {
    /// solidity: uint256 public currentEpoch;
    pub current_epoch: U256,

    /// solidity: mapping(bytes32 => uint256) public epochForHash;
    pub epoch_for_hash: BTreeMap<[u8; 32], U256>,

    /// solidity: mapping(uint256 => bytes32) public hashForEpoch;
    pub hash_for_epoch: BTreeMap<U256, [u8; 32]>,
}

impl AuthWeightedStateAccount {
    /// Updates the epoch and operators in the state.
    pub fn update_epoch_and_operators(
        &mut self,
        operators_hash: [u8; 32],
    ) -> Result<(), AuthWeightedError> {
        // Check: Duplicate operators.
        if self.is_duplicate_operator(&operators_hash) {
            return Err(AuthWeightedError::DuplicateOperators);
        }

        // XXX: Is this the correct way to [re]define an epoch?
        // TODO: use checked math.
        self.current_epoch = self.current_epoch + 1.into();
        self.hash_for_epoch
            .insert(self.current_epoch, operators_hash);
        self.epoch_for_hash
            .insert(operators_hash, self.current_epoch);
        Ok(())
    }

    /// Returns true if there's a duplicate operator for a given epoch.
    pub fn is_duplicate_operator(&self, operators_hash: &[u8; 32]) -> bool {
        self.epoch_for_hash.get(operators_hash).is_some()
    }

    /// Reallocate the state account data.
    pub fn reallocate<'a>(
        &self,
        state_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let data = borsh::to_vec(self)?;
        let size = data.len();
        let new_minimum_balance = Rent::get()?.minimum_balance(size);
        let lamports_diff = new_minimum_balance.saturating_sub(state_account.lamports());
        invoke(
            &system_instruction::transfer(payer.key, state_account.key, lamports_diff),
            &[payer.clone(), state_account.clone(), system_program.clone()],
        )?;
        state_account.realloc(size, false)?;
        state_account.try_borrow_mut_data()?[..size].copy_from_slice(&data);
        Ok(())
    }
}
