//! Processor for [`TokenManager`] related requests.

use axelar_rkyv_encoding::types::PublicKey;
use interchain_token_transfer_gmp::DeployTokenManager;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::program_option::COption;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};
use spl_token_2022::extension::StateWithExtensions;
use spl_token_2022::state::Mint;

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
    check_accounts(accounts)?;

    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let token_manager_ata_legacy = next_account_info(accounts_iter)?;
    let token_manager_ata_2022 = next_account_info(accounts_iter)?;
    let token_program_legacy = next_account_info(accounts_iter)?;
    let token_program_2022 = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;

    let ty: token_manager::Type = payload.token_manager_type.try_into()?;
    validate_token_type(&ty, token_mint, token_manager_pda)?;

    let (token_program, token_manager_ata) = if token_mint.owner == token_program_legacy.key {
        (token_program_legacy, token_manager_ata_legacy)
    } else {
        (token_program_2022, token_manager_ata_2022)
    };

    let Ok((operator, token_address)) = token_manager::decode_params(payload.params.as_ref())
    else {
        msg!("Failed to decode operator and token address");
        return Err(ProgramError::InvalidInstructionData);
    };

    create_associated_token_account(
        payer,
        token_mint,
        token_manager_ata,
        token_manager_pda,
        system_account,
        token_program,
    )?;

    let token_id = PublicKey::new_ed25519(payload.token_id.0);
    let (_token_manager_pda, bump) = crate::token_manager_pda(its_root_pda.key, token_id.as_ref());
    let token_manager_ata = PublicKey::new_ed25519(token_manager_ata.key.to_bytes());
    let mut operators = vec![PublicKey::new_ed25519(its_root_pda.key.to_bytes())];

    if let Some(operator) = operator {
        operators.push(operator);
    }

    let minters = match ty {
        token_manager::Type::NativeInterchainToken
        | token_manager::Type::MintBurn
        | token_manager::Type::MintBurnFrom => {
            // TODO: The mint authority set by the user, if any, is sent within the
            // `DeployInterchainToken` message.
            Some(vec![])
        }
        token_manager::Type::LockUnlock | token_manager::Type::LockUnlockFee => None,
    };

    let token_manager = TokenManager::new(
        ty,
        token_id,
        token_address,
        token_manager_ata,
        bump,
        operators,
        minters,
    );

    program_utils::init_rkyv_pda::<{ TokenManager::LEN }, _>(
        payer,
        token_manager_pda,
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

fn check_accounts(accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata_legacy = next_account_info(accounts_iter)?;
    let _token_manager_ata_2022 = next_account_info(accounts_iter)?;
    let token_program_legacy = next_account_info(accounts_iter)?;
    let token_program_2022 = next_account_info(accounts_iter)?;
    let ata_program = next_account_info(accounts_iter)?;

    if !system_program::check_id(system_account.key) {
        msg!("Invalid system account provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    if its_root_pda
        .check_initialized_pda_without_deserialization(&crate::id())
        .is_err()
    {
        msg!("ITS root PDA is not initialized");
        return Err(ProgramError::UninitializedAccount);
    }

    if token_manager_pda.check_uninitialized_pda().is_err() {
        msg!("TokenManager PDA is already initialized");
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    if spl_token_2022::check_spl_token_program_account(token_mint.owner).is_err() {
        msg!("Invalid token mint account provided");
        return Err(ProgramError::InvalidAccountData);
    }

    if token_program_legacy.key != token_mint.owner && token_program_2022.key != token_mint.owner {
        msg!("Mint and program account mismatch");
        return Err(ProgramError::IncorrectProgramId);
    }

    if !spl_associated_token_account::check_id(ata_program.key) {
        msg!("Invalid associated token account program provided");
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

fn validate_token_type(
    ty: &token_manager::Type,
    token_mint: &AccountInfo<'_>,
    token_manager_pda: &AccountInfo<'_>,
) -> ProgramResult {
    let mint_data = token_mint.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    // TODO: There's more logic required here, possibly some check on
    // the TokenManager being the delegate of some account, etc. It's still not
    // clear to me and I think it will become clearer when we start working on the
    // deployment of the token itself and the the transfers.
    match (mint.base.mint_authority, ty) {
        (
            COption::None,
            token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom
            | token_manager::Type::NativeInterchainToken,
        ) => {
            msg!("Mint authority is required for MintBurn and MintBurnFrom tokens");
            Err(ProgramError::InvalidInstructionData)
        }
        (
            COption::Some(key),
            token_manager::Type::MintBurn
            | token_manager::Type::MintBurnFrom
            | token_manager::Type::NativeInterchainToken,
        ) => {
            if &key == token_manager_pda.key {
                Ok(())
            } else {
                msg!(
                    "TokenManager is not the mint authority, which is required for this token type"
                );
                Err(ProgramError::InvalidInstructionData)
            }
        }
        _ => Ok(()),
    }
}

fn create_associated_token_account<'a>(
    payer: &AccountInfo<'a>,
    token_mint_account: &AccountInfo<'a>,
    associated_token_account: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        payer.key,
        wallet.key,
        token_mint_account.key,
        token_program.key,
    );

    invoke(
        &create_ata_ix,
        &[
            payer.clone(),
            associated_token_account.clone(),
            wallet.clone(),
            token_mint_account.clone(),
            system_account.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}
