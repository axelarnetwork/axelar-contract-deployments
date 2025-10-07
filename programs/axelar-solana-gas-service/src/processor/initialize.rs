use program_utils::pda::BytemuckedPda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use crate::state::Config;
use crate::{assert_valid_config_pda, get_config_pda, seed_prefixes};

/// This function is used to initialize a config on the program
pub(crate) fn process_initialize_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
) -> ProgramResult {
    let accounts = &mut accounts.iter();
    let payer = next_account_info(accounts)?;
    let operator = next_account_info(accounts)?;
    let config_pda = next_account_info(accounts)?;
    let system_account = next_account_info(accounts)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !operator.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (_, bump) = get_config_pda();

    // Check: Gateway Config account uses the canonical bump.
    assert_valid_config_pda(bump, config_pda.key)?;

    // Initialize the account
    program_utils::pda::init_pda_raw(
        payer,
        config_pda,
        program_id,
        system_account,
        Config::pda_size().try_into().expect("must be valid u64"),
        &[seed_prefixes::CONFIG_SEED, &[bump]],
    )?;
    let mut data = config_pda.try_borrow_mut_data()?;
    let gateway_config = Config::init_mut(&mut data).ok_or(ProgramError::InvalidAccountData)?;

    *gateway_config = Config {
        bump,
        operator: *operator.key,
    };

    Ok(())
}
