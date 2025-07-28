use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::assert_valid_config_pda;
use crate::state::Config;

/// This function is used to transfer operatorship of the gas service
pub(crate) fn process_transfer_operatorship(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let current_operator = next_account_info(accounts)?;
    let new_operator = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;

    if !current_operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    config_pda.check_initialized_pda_without_deserialization(program_id)?;

    let mut data = config_pda.try_borrow_mut_data()?;
    let config = Config::read_mut(&mut data).ok_or(ProgramError::InvalidAccountData)?;

    assert_valid_config_pda(config.bump, config_pda.key)?;

    if current_operator.key != &config.operator {
        return Err(ProgramError::InvalidAccountOwner);
    }

    config.operator = *new_operator.key;

    Ok(())
}
