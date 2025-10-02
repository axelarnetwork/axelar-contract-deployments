use crate::assert_valid_config_pda;
use crate::events::{
    NativeGasAddedEvent, NativeGasPaidForContractCallEvent, NativeGasRefundedEvent,
};
use crate::state::Config;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{
    pda::{BytemuckedPda, ValidPDA},
    transfer_lamports, validate_system_account_key,
};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_instruction;

pub(crate) fn process_pay_native_for_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    gas_fee_amount: u64,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;
    event_cpi_accounts!(accounts);

    validate_system_account_key(system_program.key)?;

    try_load_config(program_id, config_pda)?;

    invoke(
        &system_instruction::transfer(sender.key, config_pda.key, gas_fee_amount),
        &[sender.clone(), config_pda.clone(), system_program.clone()],
    )?;

    // Emit an event
    emit_cpi!(NativeGasPaidForContractCallEvent {
        config_pda: *config_pda.key,
        destination_chain,
        destination_address,
        payload_hash,
        refund_address,
        gas_fee_amount,
    });

    Ok(())
}

/// Performs all the config checks and returns the config if it is valid
fn try_load_config(
    program_id: &Pubkey,
    config_pda: &AccountInfo<'_>,
) -> Result<Config, ProgramError> {
    config_pda.check_initialized_pda_without_deserialization(program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    assert_valid_config_pda(config.bump, config_pda.key)?;
    Ok(*config)
}

pub(crate) fn add_native_gas(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    ix_index: u8,
    event_ix_index: u8,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let system_program = next_account_info(accounts)?;
    event_cpi_accounts!(accounts);

    validate_system_account_key(system_program.key)?;

    try_load_config(program_id, config_pda)?;

    invoke(
        &system_instruction::transfer(sender.key, config_pda.key, gas_fee_amount),
        &[sender.clone(), config_pda.clone(), system_program.clone()],
    )?;

    // Emit an event
    emit_cpi!(NativeGasAddedEvent {
        config_pda: *config_pda.key,
        tx_hash,
        ix_index,
        event_ix_index,
        refund_address,
        gas_fee_amount,
    });

    Ok(())
}

pub(crate) fn collect_fees_native(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
) -> ProgramResult {
    send_native(program_id, accounts, amount)?;

    Ok(())
}

pub(crate) fn refund_native(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    ix_index: u8,
    event_ix_index: u8,
    fees: u64,
) -> ProgramResult {
    send_native(program_id, accounts, fees)?;

    let accounts = &mut accounts.iter();
    let _operator = next_account_info(accounts)?;
    let receiver = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    event_cpi_accounts!(accounts);

    // Emit an event
    emit_cpi!(NativeGasRefundedEvent {
        tx_hash,
        config_pda: *config_pda.key,
        ix_index,
        event_ix_index,
        receiver: *receiver.key,
        fees,
    });

    Ok(())
}

fn send_native(program_id: &Pubkey, accounts: &[AccountInfo<'_>], amount: u64) -> ProgramResult {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let operator = next_account_info(accounts)?;
    let receiver = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;

    {
        // Check: Valid Config PDA
        let config = try_load_config(program_id, config_pda)?;

        // Check: Operator matches
        if operator.key != &config.operator {
            return Err(ProgramError::InvalidAccountOwner);
        }
    }

    // Check: Operator is signer
    if !operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    transfer_lamports(config_pda, receiver, amount)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pay_native_for_contract_call_cannot_accept_zero_amount() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let destination_chain = "destination_chain".to_owned();
        let destination_address = "destination_address".to_owned();
        let payload_hash = [0; 32];
        let refund_address = Pubkey::new_unique();
        let gas_fee_amount = 0;

        let result = process_pay_native_for_contract_call(
            &program_id,
            &accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_add_native_gas_cannot_accept_zero_amount() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let tx_hash = [0; 64];
        let ix_index = 0;
        let event_ix_index = 0;
        let gas_fee_amount = 0;
        let refund_address = Pubkey::new_unique();

        let result = add_native_gas(
            &program_id,
            &accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            refund_address,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_collect_fees_native_cannot_accept_zero_amount() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let amount = 0;

        let result = collect_fees_native(&program_id, &accounts, amount);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }
}
