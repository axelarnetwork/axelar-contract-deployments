use crate::events::{SplGasAddedEvent, SplGasPaidForContractCallEvent, SplGasRefundedEvent};
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use crate::state::Config;
use crate::{assert_valid_config_pda, seed_prefixes};

fn ensure_valid_token_account(
    token_account: &AccountInfo<'_>,
    token_program: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    wallet: &AccountInfo<'_>,
) -> ProgramResult {
    if token_account.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }
    let token_account_data =
        spl_token_2022::state::Account::unpack_from_slice(&token_account.try_borrow_data()?)?;
    if token_account_data.mint != *mint.key || token_account_data.owner != *wallet.key {
        return Err(ProgramError::InvalidAccountData);
    };
    Ok(())
}

fn ensure_valid_config_pda(config_pda: &AccountInfo<'_>, program_id: &Pubkey) -> ProgramResult {
    config_pda.check_initialized_pda_without_deserialization(program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    assert_valid_config_pda(config.bump, config_pda.key)?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn transfer_tokens(
    token_program: &AccountInfo<'_>,
    sender_token_account: &AccountInfo<'_>,
    mint: &AccountInfo<'_>,
    receiver_token_account: &AccountInfo<'_>,
    sender_authority: &AccountInfo<'_>,
    signer_pubkeys: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    spl_token_2022::instruction::transfer_checked(
        token_program.key,
        sender_token_account.key,
        mint.key,
        receiver_token_account.key,
        sender_authority.key,
        signer_pubkeys
            .iter()
            .map(|x| x.key)
            .collect::<Vec<_>>()
            .as_slice(),
        amount,
        decimals,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn process_pay_spl_for_contract_call(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    gas_fee_amount: u64,
    decimals: u8,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (accounts, signer_pubkeys) = accounts.split_at(8);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_token_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_token_account = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    event_cpi_accounts!(accounts);

    if !sender.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_token_account is owned by the Token Program and matches expected fields
    ensure_valid_token_account(config_pda_token_account, token_program, mint, config_pda)?;

    // ensure sender_pda is owned by the token program matches expected fields
    ensure_valid_token_account(sender_token_account, token_program, mint, sender)?;

    let ix = transfer_tokens(
        token_program,
        sender_token_account,
        mint,
        config_pda_token_account,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_token_account.clone(),
            config_pda_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    emit_cpi!(SplGasPaidForContractCallEvent {
        config_pda: *config_pda.key,
        config_pda_ata: *config_pda_token_account.key,
        mint: *mint.key,
        token_program_id: *token_program.key,
        destination_chain,
        destination_address,
        payload_hash,
        refund_address,
        gas_fee_amount,
    });

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn add_spl_gas(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    ix_index: u8,
    event_ix_index: u8,
    gas_fee_amount: u64,
    refund_address: Pubkey,
    decimals: u8,
) -> ProgramResult {
    if gas_fee_amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let (accounts, signer_pubkeys) = accounts.split_at(8);
    let accounts = &mut accounts.iter();
    let sender = next_account_info(accounts)?;
    let sender_token_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_token_account = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;
    event_cpi_accounts!(accounts);

    if !sender.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_token_account is owned by the Token Program and matches expected fields
    ensure_valid_token_account(config_pda_token_account, token_program, mint, config_pda)?;

    // ensure sender_pda is owned by the token program matches expected fields
    ensure_valid_token_account(sender_token_account, token_program, mint, sender)?;

    let ix = transfer_tokens(
        token_program,
        sender_token_account,
        mint,
        config_pda_token_account,
        sender,
        signer_pubkeys,
        gas_fee_amount,
        decimals,
    )?;

    invoke(
        &ix,
        &[
            sender.clone(),
            mint.clone(),
            sender_token_account.clone(),
            config_pda_token_account.clone(),
            token_program.clone(),
        ],
    )?;

    // Emit an event
    emit_cpi!(SplGasAddedEvent {
        config_pda: *config_pda.key,
        config_pda_ata: *config_pda_token_account.key,
        mint: *mint.key,
        token_program_id: *token_program.key,
        tx_hash,
        ix_index,
        event_ix_index,
        refund_address,
        gas_fee_amount,
    });

    Ok(())
}

pub(crate) fn collect_fees_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    send_spl(program_id, accounts, amount, decimals)?;

    Ok(())
}

pub(crate) fn refund_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    tx_hash: [u8; 64],
    ix_index: u8,
    event_ix_index: u8,
    fees: u64,
    decimals: u8,
) -> ProgramResult {
    send_spl(program_id, accounts, fees, decimals)?;

    let accounts_iter = &mut accounts.iter();
    let _operator = next_account_info(accounts_iter)?;
    let receiver_token_account = next_account_info(accounts_iter)?;
    let config_pda = next_account_info(accounts_iter)?;
    let config_pda_token_account = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    event_cpi_accounts!(accounts_iter);

    // Emit an event
    emit_cpi!(SplGasRefundedEvent {
        config_pda_ata: *config_pda_token_account.key,
        mint: *mint.key,
        token_program_id: *token_program.key,
        tx_hash,
        config_pda: *config_pda.key,
        ix_index,
        event_ix_index,
        receiver: *receiver_token_account.key,
        fees,
    });

    Ok(())
}

fn send_spl(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    amount: u64,
    decimals: u8,
) -> ProgramResult {
    if amount == 0 {
        msg!("Gas fee amount cannot be zero");
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts = &mut accounts.iter();
    let operator = next_account_info(accounts)?;
    let receiver_token_account = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let config_pda_token_account = next_account_info(accounts)?;
    let mint = next_account_info(accounts)?;
    let token_program = next_account_info(accounts)?;

    // Ensure config_pda is valid
    ensure_valid_config_pda(config_pda, program_id)?;
    let data = config_pda.try_borrow_data()?;
    let config = Config::read(&data).ok_or(ProgramError::InvalidAccountData)?;
    // Check: Operator matches
    if operator.key != &config.operator {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Check: Operator is signer
    if !operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // valid token program
    spl_token_2022::check_spl_token_program_account(token_program.key)?;

    // ensure config_pda_token_account is owned by the Token Program and matches expected fields
    ensure_valid_token_account(config_pda_token_account, token_program, mint, config_pda)?;

    let ix = transfer_tokens(
        token_program,
        config_pda_token_account,
        mint,
        receiver_token_account,
        config_pda,
        &[],
        amount,
        decimals,
    )?;

    invoke_signed(
        &ix,
        &[
            config_pda.clone(),
            mint.clone(),
            config_pda_token_account.clone(),
            receiver_token_account.clone(),
            token_program.clone(),
        ],
        &[&[seed_prefixes::CONFIG_SEED, &[config.bump]]],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_pay_spl_for_contract_call_cannot_pay_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let destination_chain = "destination_chain".to_owned();
        let destination_address = "destination_address".to_owned();
        let payload_hash = [0; 32];
        let refund_address = Pubkey::new_unique();
        let gas_fee_amount = 0;
        let decimals = 0;

        let result = process_pay_spl_for_contract_call(
            &program_id,
            &accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
            decimals,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_add_spl_gas_cannot_add_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let tx_hash = [0; 64];
        let ix_index = 0;
        let event_ix_index = 0;
        let gas_fee_amount = 0;
        let refund_address = Pubkey::new_unique();
        let decimals = 0;

        let result = add_spl_gas(
            &program_id,
            &accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            refund_address,
            decimals,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_collect_fees_spl_cannot_collect_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let amount = 0;
        let decimals = 0;

        let result = collect_fees_spl(&program_id, &accounts, amount, decimals);

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }

    #[test]
    fn test_refund_spl_cannot_refund_zero_gas_fee() {
        let program_id = Pubkey::new_unique();
        let accounts = vec![];
        let tx_hash = [0; 64];
        let ix_index = 0;
        let event_ix_index = 0;
        let fees = 0;
        let decimals = 0;

        let result = refund_spl(
            &program_id,
            &accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            fees,
            decimals,
        );

        assert_eq!(result, Err(ProgramError::InvalidInstructionData));
    }
}
