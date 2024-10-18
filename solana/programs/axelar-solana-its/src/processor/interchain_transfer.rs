//! Module that handles the processing of the `InterchainTransfer` ITS
//! instruction.
use interchain_token_transfer_gmp::InterchainTransfer;
use program_utils::check_rkyv_initialized_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::sysvar::Sysvar;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;

use crate::instructions::Bumps;
use crate::processor::token_manager as token_manager_processor;
use crate::seed_prefixes;
use crate::state::token_manager::TokenManager;

/// Processes an incoming [`InterchainTransfer`] GMP message.
///
/// # General Info
///
/// For incoming `InterchainTransfer` messages, the behaviour of the
/// [`NativeInterchainToken`], [`MintBurn`] and [`MintBurnFrom`]
/// [`TokenManager`]s are the same: the token is minted to the destination
/// wallet's associated token account.
///
/// As for [`LockUnlock`] and [`LockUnlockFee`] [`TokenManager`]s, they are
/// typically used in the home chain of the token, thus, if we're getting an
/// incoming message with these types of [`TokenManager`] , it means that tokens
/// are returning from another chain to the home chain (Solana), and thus, there
/// SHOULD be enough tokens locked in the [`TokenManager`]. It's the
/// responsibility of the user setting up the bridge to make sure correct token
/// manager types are used according to token supply, etc.
///
/// Specifically for [`LockUnlockFee`], we can only support it for mints with
/// the [`TransferFeeConfig`] extension. In this case the fee basis
/// configuration is set when the user creates the mint, we just need to
/// calculate the fee according to the fee configuration and call the correct
/// instruction to keep the fee withheld wherever the user defined they should be
/// withheld.
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
    use crate::state::token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let token_mint = next_account_info(accounts_iter)?;
    let token_manager_ata = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let destination_wallet = next_account_info(accounts_iter)?;
    let destination_ata = next_account_info(accounts_iter)?;

    // Limit the scope of the borrow on the token manager PDA as we need to pass it
    // into the next CPI.
    let (token_manager_type, interchain_token_pda) = {
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

        (token_manager.ty.into(), interchain_token_pda)
    };

    token_manager_processor::validate_token_manager_type(
        token_manager_type,
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

    // TODO: Add flow in

    match token_manager_type {
        NativeInterchainToken | MintBurn | MintBurnFrom => mint_to(
            token_program,
            token_mint,
            destination_ata,
            token_manager_pda,
            interchain_token_pda.as_ref(),
            bumps.token_manager_pda_bump,
            amount,
        )?,
        LockUnlock => {
            let decimals = {
                let mint_data = token_mint.try_borrow_data()?;
                let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;

                mint_state.base.decimals
            };

            let transfer_info = TransferInfo {
                token_program,
                token_mint,
                destination_ata,
                token_manager_pda,
                token_manager_ata,
                interchain_token_pda_bytes: interchain_token_pda.as_ref(),
                token_manager_pda_bump: bumps.token_manager_pda_bump,
                amount,
                decimals,
                fee: None,
            };

            transfer_to(&transfer_info)?;
        }
        LockUnlockFee => {
            let (fee, decimals) = {
                let mint_data = token_mint.try_borrow_data()?;
                let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;
                let fee_config = mint_state.get_extension::<TransferFeeConfig>()?;
                let epoch = Clock::get()?.epoch;

                (
                    fee_config
                        .calculate_epoch_fee(epoch, amount)
                        .ok_or(ProgramError::ArithmeticOverflow)?,
                    mint_state.base.decimals,
                )
            };

            let transfer_info = TransferInfo {
                token_program,
                token_mint,
                destination_ata,
                token_manager_pda,
                token_manager_ata,
                interchain_token_pda_bytes: interchain_token_pda.as_ref(),
                token_manager_pda_bump: bumps.token_manager_pda_bump,
                amount,
                decimals,
                fee: Some(fee),
            };

            transfer_with_fee_to(&transfer_info)?;
        }
    }

    Ok(())
}

fn mint_to<'a>(
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    destination_ata: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    interchain_token_pda_bytes: &[u8],
    token_manager_pda_bump: u8,
    amount: u64,
) -> ProgramResult {
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
            interchain_token_pda_bytes,
            &[token_manager_pda_bump],
        ]],
    )?;

    Ok(())
}

struct TransferInfo<'a, 'b> {
    token_program: &'b AccountInfo<'a>,
    token_mint: &'b AccountInfo<'a>,
    destination_ata: &'b AccountInfo<'a>,
    token_manager_pda: &'b AccountInfo<'a>,
    token_manager_ata: &'b AccountInfo<'a>,
    interchain_token_pda_bytes: &'b [u8],
    token_manager_pda_bump: u8,
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
}

fn transfer_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::transfer_checked(
            info.token_program.key,
            info.token_manager_ata.key,
            info.token_mint.key,
            info.destination_ata.key,
            info.token_manager_pda.key,
            &[],
            info.amount,
            info.decimals,
        )?,
        &[
            info.token_mint.clone(),
            info.token_manager_ata.clone(),
            info.token_manager_pda.clone(),
            info.destination_ata.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            info.interchain_token_pda_bytes,
            &[info.token_manager_pda_bump],
        ]],
    )?;
    Ok(())
}

fn transfer_with_fee_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::extension::transfer_fee::instruction::transfer_checked_with_fee(
            info.token_program.key,
            info.token_manager_ata.key,
            info.token_mint.key,
            info.destination_ata.key,
            info.token_manager_pda.key,
            &[],
            info.amount,
            info.decimals,
            info.fee.ok_or(ProgramError::InvalidArgument)?,
        )?,
        &[
            info.token_mint.clone(),
            info.token_manager_ata.clone(),
            info.token_manager_pda.clone(),
            info.destination_ata.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            info.interchain_token_pda_bytes,
            &[info.token_manager_pda_bump],
        ]],
    )?;
    Ok(())
}
