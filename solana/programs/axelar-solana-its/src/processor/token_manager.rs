//! Processor for [`TokenManager`] related requests.

use axelar_rkyv_encoding::types::PublicKey;
use interchain_token_transfer_gmp::DeployTokenManager;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::seed_prefixes;
use crate::state::token_manager::{self, TokenManager};

/// Processes a [`DeployTokenManager`] GMP message.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_deploy<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    program_id: &Pubkey,
    payload: &DeployTokenManager,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_account = next_account_info(accounts_iter)?;

    if !system_program::check_id(system_account.key) {
        msg!("Invalid system account provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    let ty: token_manager::Type = payload.token_manager_type.try_into()?;
    let token_address = PublicKey::new_ed25519([0; 32]); // TODO: Retrieve the token address.
    let token_id = PublicKey::new_ed25519(payload.token_id.0);
    let associated_token_account = PublicKey::new_ed25519([5; 32]); // TODO: Create the associated token account.
    let (_token_manager_pda, bump) = crate::token_manager_pda(its_root_pda.key, token_id.as_ref());

    let token_manager =
        TokenManager::new(ty, token_id, token_address, associated_token_account, bump);

    program_utils::init_rkyv_pda::<{ TokenManager::LEN }, _>(
        payer,
        token_manager_account,
        program_id,
        system_account,
        token_manager,
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.key.as_ref(),
            token_id.as_ref(),
            &[bump],
        ],
    )?;

    Ok(())
}
