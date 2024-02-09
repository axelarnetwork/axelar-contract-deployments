//! Program give token instruction.

use program_utils::init_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use spl_token::instruction::mint_to;
use spl_token::state::Account;
use {spl_associated_token_account, spl_token};

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::processor::initialize::{
    assert_gas_service_root_pda, assert_interchain_token_service_root_pda,
};
use crate::state::ITSATAPDA;
use crate::{get_interchain_token_service_associated_token_account, TokenManagerType};

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
        let mint_info = next_account_info(accounts_iter)?;
        let _token_manager_ata_info = next_account_info(accounts_iter)?;
        let destination = next_account_info(accounts_iter)?;
        let associated_token_account_info = next_account_info(accounts_iter)?;
        let interchain_token_service_root_pda_info = next_account_info(accounts_iter)?;
        let gateway_root_pda_info = next_account_info(accounts_iter)?;
        let gas_service_root_pda_info = next_account_info(accounts_iter)?;
        let spl_token_program_info = next_account_info(accounts_iter)?;
        let spl_associated_token_account_program_info = next_account_info(accounts_iter)?;
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
                destination.key,
                mint_info.key,
                program_id,
            )?;

        if its_ata_info.key != &its_ata_derived {
            return Err(InterchainTokenServiceError::InvalidITSATA)?;
        }

        // TODO: move to function
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

        // // TODO: move to function
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
                // TODO: swap to function
                if **its_ata_info.try_borrow_lamports()? == 0
                    && interchain_token_service_root_pda_info.data_len() == 0
                {
                    init_pda(
                        payer_info,
                        its_ata_info,
                        program_id,
                        system_program_info,
                        ITSATAPDA {},
                        &[
                            &interchain_token_service_root_pda_info.key.to_bytes(),
                            &destination.key.to_bytes(),
                            &mint_info.key.to_bytes(),
                            &[its_ata_bump],
                        ],
                    )?;
                }
                invoke(&spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                        payer_info.key,
                        its_ata_info.key,
                        mint_info.key,
                        spl_token_program_info.key,
                    ),
                    &[
                        payer_info.clone(),
                        its_ata_info.clone(),
                        mint_info.clone(),
                        associated_token_account_info.clone(),
                        spl_token_program_info.clone(),
                        system_program_info.clone(),
                        spl_associated_token_account_program_info.clone(),
                    ],
                )?;

                let current_delegate_amount =
                    Account::unpack(&associated_token_account_info.data.borrow())?.amount;

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
                invoke_signed(
                    &spl_token::instruction::approve(
                        spl_token_program_info.key,
                        associated_token_account_info.key,
                        destination.key,
                        its_ata_info.key,
                        &[],
                        amount + current_delegate_amount,
                    )?,
                    &[
                        spl_token_program_info.clone(),
                        associated_token_account_info.clone(),
                        destination.clone(),
                        its_ata_info.clone(),
                    ],
                    &[&[
                        &interchain_token_service_root_pda_info.key.as_ref(),
                        &destination.key.as_ref(),
                        &mint_info.key.as_ref(),
                        &[its_ata_bump],
                    ]],
                )?;
            }

            TokenManagerType::LockUnlock => {
                return Err(InterchainTokenServiceError::Unimplemented)?;
            }

            TokenManagerType::LockUnlockFee => {
                return Err(InterchainTokenServiceError::Unimplemented)?;
            }

            // TODO: Add support for `Gateway`?
            _ => {
                return Err(InterchainTokenServiceError::UnsupportedTokenManagerType)?;
            }
        }

        Ok(())
    }
}
