//! Program give token instruction.

use program_utils::{init_pda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_token::instruction::{mint_to, transfer};
use token_manager::TokenManagerType;
use {spl_associated_token_account, spl_token};

use super::Processor;
use crate::get_interchain_token_service_associated_token_account;
use crate::processor::initialize::assert_interchain_token_service_root_pda;
use crate::state::{RootPDA, ITSATAPDA};

impl Processor {
    pub(crate) fn give_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_manager_type: TokenManagerType,
        amount: u64,
    ) -> Result<(), ProgramError> {
        match token_manager_type {
            TokenManagerType::MintBurn | TokenManagerType::MintBurnFrom => {
                Self::give_token_mint_burn(*program_id, accounts, amount)
            }
            TokenManagerType::LockUnlock => {
                Self::give_token_lock_unlock(*program_id, accounts, amount)
            }
            TokenManagerType::LockUnlockFee => todo!(),
            TokenManagerType::Gateway => todo!(),
        }
    }

    /// Give token `LockUnlock`
    /// - Move tokens FROM token manager ATA
    /// - Move tokens TO destination (ITS ATA for the destination user wallet,
    ///   where the user wallet
    /// is set as delegate)
    fn give_token_lock_unlock(
        program_id: Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let interchain_token_service_root_pda = next_account_info(accounts_iter)?;
        let token_manager_ata_pda = next_account_info(accounts_iter)?;
        let owner_of_its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let mint_account_pda = next_account_info(accounts_iter)?;
        let destination_wallet = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let gas_service_root_pda = next_account_info(accounts_iter)?;

        // Programs
        let spl_token_program = next_account_info(accounts_iter)?;
        let spl_associated_token_account_program = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        let _ = interchain_token_service_root_pda.check_initialized_pda::<RootPDA>(&program_id)?;
        let its_root_bump_seed = assert_interchain_token_service_root_pda(
            interchain_token_service_root_pda,
            gateway_root_pda,
            gas_service_root_pda,
            &program_id,
        )?;

        assert_eq!(
            token_manager_ata_pda.owner, spl_token_program.key,
            "Invalid token manager ATA owner"
        );

        // TODO assert token_manager_ata_pda derived correctly

        // init PDA owned by ITS program which represents tokens owned by the user
        let (owner_of_its_ata_for_user_tokens_pda_derived, bump_seed) =
            get_interchain_token_service_associated_token_account(
                interchain_token_service_root_pda.key,
                destination_wallet.key,
                mint_account_pda.key,
                &program_id,
            )?;
        assert_eq!(
            owner_of_its_ata_for_user_tokens_pda.key, &owner_of_its_ata_for_user_tokens_pda_derived,
            "Invalid ITS ATA for user wallet"
        );
        msg!("!!!! INIT PDA");
        if **its_ata_for_user_tokens_pda.try_borrow_lamports()? == 0
            && its_ata_for_user_tokens_pda.data_len() == 0
        {
            init_pda(
                payer,
                owner_of_its_ata_for_user_tokens_pda,
                &program_id,
                system_program,
                ITSATAPDA {},
                &[
                    &interchain_token_service_root_pda.key.to_bytes(),
                    &destination_wallet.key.to_bytes(),
                    &mint_account_pda.key.to_bytes(),
                    &[bump_seed],
                ],
            )?;
        }

        // idempotent initialization for its_ata_for_user_owner_pda
        msg!("!!!! INIT ATA PDA");
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                payer.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
                spl_token_program.key,
            ),
            &[
                payer.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                spl_token_program.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
            ],
        )?;

        msg!("!!!! SET DELEGATE");
        // Set Delegate to `destination_wallet` so user can withdraw the tokens whenever
        // they want
        invoke_signed(
            &spl_token::instruction::approve(
                spl_token_program.key,
                its_ata_for_user_tokens_pda.key,
                destination_wallet.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                &[],
                u64::MAX,
            )
            .unwrap(),
            &[
                payer.clone(),
                destination_wallet.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                spl_token_program.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
            ],
            &[&[
                &interchain_token_service_root_pda.key.to_bytes(),
                &destination_wallet.key.to_bytes(),
                &mint_account_pda.key.to_bytes(),
                &[bump_seed],
            ]],
        )?;

        msg!("!!!!Transfer tokens");
        // Transfer the actual tokens from the token manager ATA to the user's ITS ATA
        invoke_signed(
            &transfer(
                spl_token_program.key,
                token_manager_ata_pda.key,
                its_ata_for_user_tokens_pda.key,
                interchain_token_service_root_pda.key,
                &[],
                amount,
            )?,
            &[
                payer.clone(),
                interchain_token_service_root_pda.clone(),
                token_manager_ata_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                spl_token_program.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
            ],
            &[&[
                &gateway_root_pda.key.to_bytes(),
                &gas_service_root_pda.key.to_bytes(),
                &[its_root_bump_seed],
            ]],
        )?;

        Ok(())
    }

    /// Give token `MintBurn`
    /// - Mint tokens to ITS ATA
    /// - Wallet Address is set as delegate
    fn give_token_mint_burn(
        program_id: Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let interchain_token_service_root_pda = next_account_info(accounts_iter)?;
        let owner_of_its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let mint_account_pda = next_account_info(accounts_iter)?;
        let delegate_authority = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let gas_service_root_pda = next_account_info(accounts_iter)?;

        // Programs
        let spl_token_program = next_account_info(accounts_iter)?;
        let spl_associated_token_account_program = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        interchain_token_service_root_pda.check_initialized_pda::<RootPDA>(&program_id)?;
        let its_root_pda_bump = assert_interchain_token_service_root_pda(
            interchain_token_service_root_pda,
            gateway_root_pda,
            gas_service_root_pda,
            &program_id,
        )?;

        let (
            owner_of_its_ata_for_user_tokens_pda_derived,
            owner_of_its_ata_for_user_tokens_pda_bump,
        ) = get_interchain_token_service_associated_token_account(
            interchain_token_service_root_pda.key,
            delegate_authority.key,
            mint_account_pda.key,
            &program_id,
        )?;

        assert_eq!(
            owner_of_its_ata_for_user_tokens_pda.key, &owner_of_its_ata_for_user_tokens_pda_derived,
            "Invalid ITS ATA for user wallet"
        );

        let its_ata_for_user_tokens_pda_derived =
            spl_associated_token_account::get_associated_token_address(
                owner_of_its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
            );

        assert_eq!(
            its_ata_for_user_tokens_pda.key, &its_ata_for_user_tokens_pda_derived,
            "Invalid ITS ATA for user wallet"
        );

        msg!("!!!! INIT PDA");
        if **its_ata_for_user_tokens_pda.try_borrow_lamports()? == 0
            && its_ata_for_user_tokens_pda.data_len() == 0
        {
            init_pda(
                payer,
                owner_of_its_ata_for_user_tokens_pda,
                &program_id,
                system_program,
                ITSATAPDA {},
                &[
                    &interchain_token_service_root_pda.key.to_bytes(),
                    &delegate_authority.key.to_bytes(),
                    &mint_account_pda.key.to_bytes(),
                    &[owner_of_its_ata_for_user_tokens_pda_bump],
                ],
            )?;
        }

        // idempotent initialization for its_ata_for_user_owner_pda
        msg!("!!!! INIT ATA PDA");
        invoke(
            &spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                payer.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
                spl_token_program.key,
            ),
            &[
                payer.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
                spl_token_program.clone(),
                system_program.clone(),
                spl_associated_token_account_program.clone(),
            ],
        )?;

        msg!("!!!! MINT TO");
        invoke_signed(
            &mint_to(
                spl_token_program.key,
                mint_account_pda.key,
                its_ata_for_user_tokens_pda.key,
                interchain_token_service_root_pda.key,
                &[],
                amount,
            )?,
            &[
                spl_token_program.clone(),
                mint_account_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
                interchain_token_service_root_pda.clone(),
            ],
            &[&[
                &gateway_root_pda.key.to_bytes(),
                &gas_service_root_pda.key.to_bytes(),
                &[its_root_pda_bump],
            ]],
        )?;

        msg!("!!!! SET DELEGATE");
        invoke_signed(
            &spl_token::instruction::approve(
                spl_token_program.key,
                its_ata_for_user_tokens_pda.key,
                delegate_authority.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                &[],
                u64::MAX,
            )
            .unwrap(),
            &[
                delegate_authority.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                spl_token_program.clone(),
            ],
            &[&[
                &interchain_token_service_root_pda.key.to_bytes(),
                &delegate_authority.key.to_bytes(),
                &mint_account_pda.key.to_bytes(),
                &[owner_of_its_ata_for_user_tokens_pda_bump],
            ]],
        )?;

        Ok(())
    }
}
