//! Program give token instruction.

use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::init_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;
use solana_program::{system_instruction, system_program};
use spl_token::instruction::mint_to;
use {spl_associated_token_account, spl_token};

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::processor::initialize::{
    assert_gas_service_root_pda, assert_interchain_token_service_root_pda,
};
use crate::state::ThePDA;
use crate::TokenManagerType;

impl Processor {
    #[allow(unreachable_patterns)]
    pub(crate) fn give_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_manager_type: TokenManagerType,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer_info = next_account_info(accounts_iter)?;
        // Token Address
        let mint_info = next_account_info(accounts_iter)?;
        let _token_manager_info = next_account_info(accounts_iter)?;
        // Owner of the Associated Token Account
        let wallet_info = next_account_info(accounts_iter)?;
        // Where to send tokens / Destination
        let associated_token_account_info = next_account_info(accounts_iter)?;
        // Mint Authority
        let interchain_token_service_root_pda_info = next_account_info(accounts_iter)?;
        // Used to derive ITS PDA
        let gateway_root_pda_info = next_account_info(accounts_iter)?;
        let gas_service_root_pda_info = next_account_info(accounts_iter)?;
        // System programs
        let spl_token_program_info = next_account_info(accounts_iter)?;
        let spl_associated_token_account_program_info = next_account_info(accounts_iter)?;
        let system_program_info = next_account_info(accounts_iter)?;
        let the_pda_info = next_account_info(accounts_iter)?;

        if !spl_token::check_id(spl_token_program_info.key) {
            return Err(InterchainTokenServiceError::InvalidSPLTokenProgram)?;
        };

        if !system_program::check_id(system_program_info.key) {
            return Err(InterchainTokenServiceError::InvalidSystemAccount)?;
        };

        // TODO: separate function?
        let (the_pda_derived, the_pda_bump) = Pubkey::find_program_address(
            &[
                &interchain_token_service_root_pda_info.key.as_ref(),
                &wallet_info.key.as_ref(),
                &mint_info.key.as_ref(),
            ],
            program_id,
        );

        // TODO: move to separate function
        // crete ThePDA account if it doesn't exist
        if **the_pda_info.try_borrow_lamports()? == 0
            && interchain_token_service_root_pda_info
                .try_borrow_data()
                .unwrap()
                .len()
                == 0
        {
            init_pda(
                payer_info,
                the_pda_info,
                program_id,
                system_program_info,
                ThePDA {},
                &[
                    &interchain_token_service_root_pda_info.key.to_bytes(),
                    &wallet_info.key.to_bytes(),
                    &mint_info.key.to_bytes(),
                    &[the_pda_bump],
                ],
            )?;
        }

        if the_pda_info.key != &the_pda_derived {
            return Err(InterchainTokenServiceError::InvalidThePDA)?;
        }

        // if provided associated token account doesn't exist; create it
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                payer_info.key,
                the_pda_info.key,
                mint_info.key,
                spl_token_program_info.key,
            ),
            &[
                payer_info.clone(),
                the_pda_info.clone(),
                mint_info.clone(),
                associated_token_account_info.clone(),
                spl_token_program_info.clone(),
                system_program_info.clone(),
                spl_associated_token_account_program_info.clone(),
            ],
        )?;

        // TODO: move to separate function
        if **interchain_token_service_root_pda_info.try_borrow_lamports()? == 0
            && interchain_token_service_root_pda_info
                .try_borrow_data()
                .unwrap()
                .len()
                == 0
            && interchain_token_service_root_pda_info.owner == program_id
        {
            return Err(InterchainTokenServiceError::UninitializedITSRootPDA)?;
        }

        // // TODO: move to separate function
        if **mint_info.try_borrow_lamports()? == 0
            && mint_info.try_borrow_data()?.len() == 0
            && mint_info.owner != spl_token_program_info.key
        {
            return Err(InterchainTokenServiceError::UninitializedMintAccount)?;
        }

        assert_gas_service_root_pda(gas_service_root_pda_info);

        // // TODO: Check if token manager type is associated with the token manager.

        let bump_seed = assert_interchain_token_service_root_pda(
            interchain_token_service_root_pda_info,
            gateway_root_pda_info,
            gas_service_root_pda_info,
            program_id,
        )?;

        match token_manager_type {
            TokenManagerType::MintBurn | TokenManagerType::MintBurnFrom => {
                invoke_signed(
                    &mint_to(
                        spl_token_program_info.key,
                        mint_info.key,
                        associated_token_account_info.key,
                        interchain_token_service_root_pda_info.key,
                        &[],
                        amount,
                    )?,
                    &[
                        spl_token_program_info.clone(),
                        mint_info.clone(),
                        associated_token_account_info.clone(),
                        interchain_token_service_root_pda_info.clone(),
                    ],
                    &[&[
                        &gateway_root_pda_info.key.to_bytes(),
                        &gas_service_root_pda_info.key.to_bytes(),
                        &[bump_seed],
                    ]],
                )?;

                // // Set delegate to wallet address
                invoke_signed(
                    &spl_token::instruction::approve(
                        spl_token_program_info.key,
                        associated_token_account_info.key,
                        wallet_info.key,
                        the_pda_info.key,
                        &[],
                        amount,
                    )?,
                    &[
                        spl_token_program_info.clone(),
                        associated_token_account_info.clone(),
                        wallet_info.clone(),
                        the_pda_info.clone(),
                    ],
                    &[&[
                        &interchain_token_service_root_pda_info.key.as_ref(),
                        &wallet_info.key.as_ref(),
                        &mint_info.key.as_ref(),
                        &[the_pda_bump],
                    ]],
                )?;
            }

            TokenManagerType::LockUnlock => {
                return Err(InterchainTokenServiceError::Unimplemented)?;
            }

            TokenManagerType::LockUnlockFee => {
                return Err(InterchainTokenServiceError::Unimplemented)?;
            }

            _ => {
                return Err(InterchainTokenServiceError::UnsupportedTokenManagerType)?;
            }
        }

        Ok(())
    }
}
