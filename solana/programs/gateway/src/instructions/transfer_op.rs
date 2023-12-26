//! Transfer Operatorship instruction.

use auth_weighted::types::account::state::AuthWeightedStateAccount;
use auth_weighted::types::account::transfer_operatorship::TransferOperatorshipAccount;
use auth_weighted::types::address::Address;
use auth_weighted::types::u256::U256;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{keccak, system_instruction};

use crate::error::GatewayError;
use crate::events::emit_operatorship_transferred_event;
use crate::instructions::GatewayInstruction;
use crate::{check_program_account, cmp_addr, find_root_pda};

/// Creates a [`GatewayInstructon::TransferOperatorship`] instruction
pub fn transfer_operatorship_ix(
    payer: &Pubkey,
    new_operators: &Pubkey,
    state: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*new_operators, false),
        AccountMeta::new(*state, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = borsh::to_vec(&GatewayInstruction::TransferOperatorship {})?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

// TODO: move to processor module
pub(crate) fn transfer_operatorship(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> Result<(), ProgramError> {
    check_program_account(*program_id)?;

    let accounts_iter = &mut accounts.iter();

    let payer_info = next_account_info(accounts_iter)?;
    let new_operators_info = next_account_info(accounts_iter)?;
    let state_info = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    let (expected_pda_info, _bump) = find_root_pda();
    cmp_addr(state_info, expected_pda_info)?;

    let new_operators_data_ref: &[u8] = &new_operators_info.data.borrow();
    let new_operators_data_unpacked =
        match TransferOperatorshipAccount::unpack(new_operators_data_ref) {
            Ok(v) => v,
            Err(e) => return Err(e.into()),
        };
    let operators_length = new_operators_data_unpacked.operators_len();
    let weights_length = new_operators_data_unpacked.weights_len();

    if operators_length == 0
        || !is_sorted_asc_and_contains_no_duplicate(new_operators_data_unpacked.operators())
    {
        return Err(GatewayError::InvalidOperators.into());
    }

    if weights_length != operators_length {
        return Err(GatewayError::InvalidWeights.into());
    }

    // Accumulate weights from operators.
    let mut total_weight = U256::from(0);

    for weight in new_operators_data_unpacked.weights() {
        total_weight = total_weight + weight.clone()
    }

    if new_operators_data_unpacked.threshold() == &U256::from(0)
        || total_weight < new_operators_data_unpacked.threshold().clone()
    {
        return Err(GatewayError::InvalidThreshold.into());
    }

    let new_operators_hash = keccak::hash(new_operators_data_ref).to_bytes();

    let state_data_ref = state_info.try_borrow_mut_data()?;
    let mut state_data_unpacked = AuthWeightedStateAccount::unpack(&state_data_ref)?;

    if state_data_unpacked
        .epoch_for_hash
        .get(&new_operators_hash)
        .is_some()
    {
        return Err(GatewayError::DuplicateOperators.into());
    }

    let epoch = state_data_unpacked.current_epoch + U256::from(1);

    state_data_unpacked.current_epoch = epoch.clone();
    state_data_unpacked
        .hash_for_epoch
        .insert(epoch.clone(), new_operators_hash);
    state_data_unpacked
        .epoch_for_hash
        .insert(new_operators_hash, epoch);

    let state_data_packed = state_data_unpacked.pack();

    // hax to get around borrow checker
    drop(state_data_ref);

    // Resize state account space.
    let new_size = state_data_packed.len();
    let rent = Rent::get()?;
    let new_minimum_balance = rent.minimum_balance(new_size);
    let lamports_diff = new_minimum_balance.saturating_sub(state_info.lamports());
    invoke(
        &system_instruction::transfer(payer_info.key, state_info.key, lamports_diff),
        &[
            payer_info.clone(),
            state_info.clone(),
            system_program.clone(),
        ],
    )?;
    state_info.realloc(state_data_packed.len(), false)?;

    let mut state_data_ref = state_info.try_borrow_mut_data()?;
    state_data_ref[..state_data_packed.len()].copy_from_slice(&state_data_packed);

    emit_operatorship_transferred_event(*new_operators_info.key)?;
    Ok(())
}

/// Checks if the given list of accounts is sorted in ascending order and
/// contains no duplicates.
pub fn is_sorted_asc_and_contains_no_duplicate(addresses: &[Address]) -> bool {
    addresses.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sorted_asc_and_contains_no_duplicate() {
        // valid one
        let addresses1 = vec![
            Address::new(vec![1, 2, 3]),
            Address::new(vec![2, 3, 4]),
            Address::new(vec![3, 4, 5]),
        ];
        assert!(is_sorted_asc_and_contains_no_duplicate(&addresses1));

        // // not sorted
        let addresses2 = vec![
            Address::new(vec![3, 4, 5]),
            Address::new(vec![2, 3, 4]),
            Address::new(vec![1, 2, 3]),
        ];
        assert!(!is_sorted_asc_and_contains_no_duplicate(&addresses2));

        // duplicates
        let addresses3 = vec![
            Address::new(vec![1, 2, 3]),
            Address::new(vec![2, 3, 4]),
            Address::new(vec![2, 3, 4]),
        ];
        assert!(!is_sorted_asc_and_contains_no_duplicate(&addresses3));
    }
}
