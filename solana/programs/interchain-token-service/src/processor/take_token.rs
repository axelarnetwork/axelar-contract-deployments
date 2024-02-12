//! Program take token instruction.

use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_token;
use spl_token::instruction::burn;

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::processor::initialize::assert_interchain_token_service_root_pda;
use crate::state::RootPDA;
use crate::{get_interchain_token_service_associated_token_account, TokenManagerType};

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
            TokenManagerType::LockUnlock => todo!(),
            TokenManagerType::LockUnlockFee => todo!(),
        }
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

        interchain_token_service_root_pda.check_initialized_pda::<RootPDA>(&program_id)?;
        assert_interchain_token_service_root_pda(
            interchain_token_service_root_pda,
            gateway_root_pda,
            gas_service_root_pda,
            &crate::id(),
        )?;

        let (
            owner_of_its_ata_for_user_tokens_pda_derived,
            owner_of_its_ata_for_user_tokens_pda_bump,
        ) = get_interchain_token_service_associated_token_account(
            interchain_token_service_root_pda.key,
            delegate_authority.key,
            mint_account_pda.key,
            &crate::id(),
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
