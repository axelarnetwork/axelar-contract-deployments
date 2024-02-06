//! Program take token instruction.

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use spl_token;
use spl_token::instruction::burn;
use spl_token::state::Account;

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::processor::initialize::{
    assert_gas_service_root_pda, assert_interchain_token_service_root_pda,
};
use crate::{get_interchain_token_service_associated_token_account, TokenManagerType};

impl Processor {
    #[allow(unreachable_patterns)]
    pub(crate) fn take_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_manager_type: TokenManagerType,
        amount: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let _payer_info = next_account_info(accounts_iter)?;
        let mint_info = next_account_info(accounts_iter)?;
        let _token_manager_info = next_account_info(accounts_iter)?;
        // aka wallet address
        let its_ata_delegate_authority_info = next_account_info(accounts_iter)?;
        let associated_token_account_info = next_account_info(accounts_iter)?;
        let interchain_token_service_root_pda_info = next_account_info(accounts_iter)?;
        let gateway_root_pda_info = next_account_info(accounts_iter)?;
        let gas_service_root_pda_info = next_account_info(accounts_iter)?;
        let spl_token_program_info = next_account_info(accounts_iter)?;
        let _spl_associated_token_account_program_info = next_account_info(accounts_iter)?;
        let system_program_info = next_account_info(accounts_iter)?;
        let its_ata_info = next_account_info(accounts_iter)?;

        if !spl_token::check_id(spl_token_program_info.key) {
            return Err(InterchainTokenServiceError::InvalidSPLTokenProgram)?;
        };

        if !system_program::check_id(system_program_info.key) {
            return Err(InterchainTokenServiceError::InvalidSystemAccount)?;
        };

        let (its_ata_derived, its_ata_bump) =
            get_interchain_token_service_associated_token_account(
                interchain_token_service_root_pda_info.key,
                its_ata_delegate_authority_info.key,
                mint_info.key,
                program_id,
            )?;

        if its_ata_info.key != &its_ata_derived {
            return Err(InterchainTokenServiceError::InvalidITSATA)?;
        }

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

        let _bump_seed = assert_interchain_token_service_root_pda(
            interchain_token_service_root_pda_info,
            gateway_root_pda_info,
            gas_service_root_pda_info,
            program_id,
        )?;

        match token_manager_type {
            TokenManagerType::MintBurn => {
                let current_delegate_amount =
                    Account::unpack(&associated_token_account_info.data.borrow())?.amount;

                invoke_signed(
                    &burn(
                        spl_token_program_info.key,
                        associated_token_account_info.key,
                        mint_info.key,
                        its_ata_info.key,
                        &[],
                        amount,
                    )?,
                    &[
                        spl_token_program_info.clone(),
                        associated_token_account_info.clone(),
                        mint_info.clone(),
                        interchain_token_service_root_pda_info.clone(),
                        its_ata_info.clone(),
                    ],
                    &[&[
                        &interchain_token_service_root_pda_info.key.as_ref(),
                        &its_ata_delegate_authority_info.key.as_ref(),
                        &mint_info.key.as_ref(),
                        &[its_ata_bump],
                    ]],
                )?;

                // Update delegate authority with amount allowance
                invoke_signed(
                    &spl_token::instruction::approve(
                        spl_token_program_info.key,
                        associated_token_account_info.key,
                        its_ata_delegate_authority_info.key,
                        its_ata_info.key,
                        &[],
                        current_delegate_amount - amount,
                    )?,
                    &[
                        spl_token_program_info.clone(),
                        associated_token_account_info.clone(),
                        its_ata_delegate_authority_info.clone(),
                        its_ata_info.clone(),
                    ],
                    &[&[
                        &interchain_token_service_root_pda_info.key.as_ref(),
                        &its_ata_delegate_authority_info.key.as_ref(),
                        &mint_info.key.as_ref(),
                        &[its_ata_bump],
                    ]],
                )?;
            }

            TokenManagerType::MintBurnFrom => {
                return Err(InterchainTokenServiceError::Unimplemented)?;
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
