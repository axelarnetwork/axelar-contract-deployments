//! Program take token instruction.

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_token::instruction::{burn, transfer};
use token_manager::TokenManagerType;

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::get_interchain_token_service_associated_token_account;
use crate::processor::assert_root_its_derivation;
use crate::state::RootPDA;

impl Processor {
    pub(crate) fn take_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_manager_type: TokenManagerType,
        amount: u64,
    ) -> Result<(), ProgramError> {
        match token_manager_type {
            TokenManagerType::MintBurn => Self::take_token_mint_burn(*program_id, accounts, amount),
            TokenManagerType::MintBurnFrom => todo!(),
            TokenManagerType::LockUnlock => {
                Self::take_token_lock_unlock(*program_id, accounts, amount)
            }
            TokenManagerType::LockUnlockFee => todo!(),
        }
    }

    /// Take token `LockUnlock`
    /// - Move tokens TO token manager ATA from user ITS ATA
    fn take_token_lock_unlock(
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

        let root_pda =
            interchain_token_service_root_pda.check_initialized_pda::<RootPDA>(&program_id)?;
        assert_root_its_derivation(
            gateway_root_pda,
            gas_service_root_pda,
            &root_pda,
            interchain_token_service_root_pda,
        )?;

        assert_eq!(
            token_manager_ata_pda.owner, spl_token_program.key,
            "Invalid token manager ATA owner"
        );

        // TODO assert token_manager_ata_pda derived correctly

        // init PDA owned by ITS program which represents tokens owned by the user
        let (
            owner_of_its_ata_for_user_tokens_pda_derived,
            owner_of_its_ata_for_user_tokens_pda_bump,
        ) = get_interchain_token_service_associated_token_account(
            interchain_token_service_root_pda.key,
            destination_wallet.key,
            mint_account_pda.key,
            &program_id,
        )?;
        assert_eq!(
            owner_of_its_ata_for_user_tokens_pda.key, &owner_of_its_ata_for_user_tokens_pda_derived,
            "Invalid ITS ATA for user wallet"
        );

        msg!("!!!! Transfer");
        // Transfer the actual tokens from user ITS ATA to the the token manager ATA
        invoke_signed(
            &transfer(
                spl_token_program.key,
                its_ata_for_user_tokens_pda.key, /* INFO: This is user ITS Owned ATA for User
                                                  * Tokens (User is Delegate) */
                token_manager_ata_pda.key, // INFO: This is Vault for Locking funds
                owner_of_its_ata_for_user_tokens_pda.key,
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
                &interchain_token_service_root_pda.key.to_bytes(),
                &destination_wallet.key.to_bytes(),
                &mint_account_pda.key.to_bytes(),
                &[owner_of_its_ata_for_user_tokens_pda_bump],
            ]],
        )?;

        Ok(())
    }

    /// Take token `MintBurn`
    /// - Burn tokens from ITS ATA
    fn take_token_mint_burn(
        program_id: Pubkey,
        accounts: &[AccountInfo],
        amount: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let _payer = next_account_info(accounts_iter)?;
        let interchain_token_service_root_pda = next_account_info(accounts_iter)?;
        let owner_of_its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let its_ata_for_user_tokens_pda = next_account_info(accounts_iter)?;
        let mint_account_pda = next_account_info(accounts_iter)?;
        let delegate_authority = next_account_info(accounts_iter)?;
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let gas_service_root_pda = next_account_info(accounts_iter)?;

        // Programs
        let spl_token_program = next_account_info(accounts_iter)?;

        let root_pda =
            interchain_token_service_root_pda.check_initialized_pda::<RootPDA>(&crate::id())?;
        assert_root_its_derivation(
            gateway_root_pda,
            gas_service_root_pda,
            &root_pda,
            interchain_token_service_root_pda,
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

        if **its_ata_for_user_tokens_pda.try_borrow_lamports()? == 0
            && its_ata_for_user_tokens_pda.data_len() == 0
        {
            return Err(InterchainTokenServiceError::InvalidITSATA)?;
        }

        msg!("!!!! BURN");
        invoke_signed(
            &burn(
                spl_token_program.key,
                its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                &[],
                amount,
            )?,
            &[
                spl_token_program.clone(),
                mint_account_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
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
