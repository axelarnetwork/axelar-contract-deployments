//! Module that handles the processing of the `InterchainTransfer` ITS
//! instruction.
use interchain_token_transfer_gmp::InterchainTransfer;
use program_utils::check_rkyv_initialized_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;

use crate::instructions::Bumps;
use crate::processor::token_manager as token_manager_processor;
use crate::seed_prefixes;
use crate::state::token_manager::TokenManager;

/// Processes a [`InterchainTransfer`] GMP message.
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_transfer<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    payload: &InterchainTransfer,
    bumps: Bumps,
) -> ProgramResult {
    let Ok(converted_amount) = payload.amount.try_into() else {
        msg!("Failed to convert amount");
        return Err(ProgramError::InvalidInstructionData);
    };

    give_token(payer, accounts, converted_amount, bumps)?;

    Ok(())
}

fn give_token<'a>(
    payer: &AccountInfo<'a>,
    accounts: &[AccountInfo<'a>],
    amount: u64,
    bumps: Bumps,
) -> ProgramResult {
    use crate::state::token_manager::ArchivedType::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let destination_wallet = next_account_info(accounts_iter)?;
    let destination_ata = next_account_info(accounts_iter)?;

    let token_manager_pda_data = token_manager_pda.try_borrow_data()?;
    let token_manager = check_rkyv_initialized_pda::<TokenManager>(
        &crate::id(),
        token_manager_pda,
        token_manager_pda_data.as_ref(),
    )?;
    let (interchain_token_pda, _) = crate::create_interchain_token_pda(
        its_root_pda.key,
        token_manager.token_id.as_ref(),
        bumps.interchain_token_pda_bump,
    );

    // TODO: Add flow in

    match token_manager.ty {
        NativeInterchainToken | MintBurn | MintBurnFrom => {
            token_manager_processor::validate_token_manager_type(
                token_manager.ty.into(),
                token_mint,
                token_manager_pda,
            )?;

            crate::create_associated_token_account_idempotent(
                payer,
                token_mint,
                destination_ata,
                destination_wallet,
                system_account,
                token_program,
            )?;

            invoke_signed(
                &spl_token_2022::instruction::mint_to(
                    token_program.key,
                    token_mint.key,
                    destination_ata.key,
                    token_manager_pda.key,
                    &[],
                    amount,
                )?,
                &[
                    token_mint.clone(),
                    destination_ata.clone(),
                    token_manager_pda.clone(),
                ],
                &[&[
                    seed_prefixes::TOKEN_MANAGER_SEED,
                    interchain_token_pda.as_ref(),
                    &[bumps.token_manager_pda_bump],
                ]],
            )?;
        }
        LockUnlock => {
            // TODO: Transfer token from token_manager_ata to the destination
            return Err(ProgramError::InvalidSeeds);
        }
        LockUnlockFee => {
            // TODO: Transfer token from token_manager_ata to the destination with a fee
            return Err(ProgramError::InvalidArgument);
        }
    }

    Ok(())
}
