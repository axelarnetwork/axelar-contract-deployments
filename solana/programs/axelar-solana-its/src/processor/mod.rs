//! Program state processor

use axelar_executable::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_rkyv_encoding::types::GmpMetadata;
use interchain_token_transfer_gmp::GMPPayload;
use itertools::Itertools;
use program_utils::{check_rkyv_initialized_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::check_program_account;
use crate::instructions::{
    derive_its_accounts, its_account_indices, Bumps, InterchainTokenServiceInstruction,
};
use crate::state::token_manager::TokenManager;
use crate::state::InterchainTokenService;

pub mod interchain_token;
pub mod token_manager;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    ///
    /// # Errors
    ///
    /// A `ProgramError` containing the error that occurred is returned. Log
    /// messages are also generated with more detailed information.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        instruction_data: &[u8],
    ) -> ProgramResult {
        check_program_account(*program_id)?;
        let instruction = match InterchainTokenServiceInstruction::from_bytes(instruction_data) {
            Ok(instruction) => instruction,
            Err(err) => {
                msg!("Failed to deserialize instruction: {:?}", err);
                return Err(ProgramError::InvalidInstructionData);
            }
        };

        match instruction {
            InterchainTokenServiceInstruction::Initialize { pda_bump } => {
                process_initialize(accounts, pda_bump)?;
            }
            InterchainTokenServiceInstruction::ItsGmpPayload {
                abi_payload,
                gmp_metadata,
                bumps,
            } => {
                process_its_gmp_payload(accounts, gmp_metadata, &abi_payload, bumps)?;
            }
        }

        Ok(())
    }
}

fn process_initialize(accounts: &[AccountInfo<'_>], pda_bump: u8) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let its_root_pda = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: PDA Account is not initialized
    its_root_pda.check_uninitialized_pda()?;

    // Check the bump seed is correct
    crate::check_initialization_bump(pda_bump, its_root_pda.key, gateway_root_pda.key)?;
    let data = InterchainTokenService::new(pda_bump);

    program_utils::init_rkyv_pda::<{ InterchainTokenService::LEN }, _>(
        payer,
        its_root_pda,
        &crate::id(),
        system_account,
        data,
        &[
            crate::seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[pda_bump],
        ],
    )?;

    Ok(())
}

fn process_its_gmp_payload(
    accounts: &[AccountInfo<'_>],
    gmp_metadata: GmpMetadata,
    abi_payload: &[u8],
    bumps: Bumps,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    validate_with_gmp_metadata(&crate::id(), gateway_accounts, gmp_metadata, abi_payload)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;

    let payload =
        GMPPayload::decode(abi_payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    validate_its_accounts(instruction_accounts, gateway_root_pda.key, &payload, bumps)?;

    match payload {
        GMPPayload::InterchainTransfer(_interchain_token_transfer) => {
            msg!("Received InterchainTransfer message");
        }
        GMPPayload::DeployInterchainToken(deploy_interchain_token) => {
            interchain_token::process_deploy(
                payer,
                instruction_accounts,
                deploy_interchain_token,
                bumps,
            )?;
        }
        GMPPayload::DeployTokenManager(deploy_token_manager) => {
            token_manager::process_deploy(
                payer,
                instruction_accounts,
                &deploy_token_manager,
                bumps,
            )?;
        }
    }
    Ok(())
}

fn validate_its_accounts(
    accounts: &[AccountInfo<'_>],
    gateway_root_pda: &Pubkey,
    payload: &GMPPayload,
    bumps: Bumps,
) -> ProgramResult {
    // In this case we cannot derive the mint account, so we just use what we got
    // and check later against the mint within the `TokenManager` PDA.
    let maybe_mint = if let GMPPayload::InterchainTransfer(_) = payload {
        accounts
            .get(its_account_indices::TOKEN_MINT_INDEX)
            .map(|account| *account.key)
    } else {
        None
    };

    let token_program = accounts
        .get(its_account_indices::TOKEN_PROGRAM_INDEX)
        .map(|account| *account.key)
        .ok_or(ProgramError::InvalidAccountData)?;

    let (derived_its_accounts, new_bumps) = derive_its_accounts(
        gateway_root_pda,
        payload,
        token_program,
        maybe_mint,
        Some(bumps),
    )?;

    if new_bumps != bumps {
        return Err(ProgramError::InvalidAccountData);
    }

    for element in accounts.iter().zip_longest(derived_its_accounts.iter()) {
        match element {
            itertools::EitherOrBoth::Both(provided, derived) => {
                if provided.key != &derived.pubkey {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            itertools::EitherOrBoth::Left(_) | itertools::EitherOrBoth::Right(_) => {
                return Err(ProgramError::InvalidAccountData)
            }
        }
    }

    // Now we validate the mint account passed for `InterchainTransfer`
    if let Some(mint) = maybe_mint {
        let token_manager_pda = accounts
            .get(its_account_indices::TOKEN_MANAGER_PDA_INDEX)
            .ok_or(ProgramError::InvalidAccountData)?;
        let token_manager_pda_data = token_manager_pda.try_borrow_data()?;

        let token_manager = check_rkyv_initialized_pda::<TokenManager>(
            &crate::id(),
            token_manager_pda,
            token_manager_pda_data.as_ref(),
        )?;

        if token_manager.token_address.as_ref() != mint.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    Ok(())
}
