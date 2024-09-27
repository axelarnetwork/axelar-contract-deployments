//! Program state processor

use axelar_executable::{validate_flattened_message, PROGRAM_ACCOUNTS_START_INDEX};
use interchain_token_transfer_gmp::GMPPayload;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use crate::instructions::InterchainTokenServiceInstruction;
use crate::{check_program_account, state};

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
                process_initialize(program_id, accounts, pda_bump)?;
            }
            InterchainTokenServiceInstruction::ItsGmpPayload {
                abi_payload,
                cross_chain_id,
                source_address,
                destination_address,
                destination_chain,
            } => {
                validate_flattened_message(
                    program_id,
                    accounts,
                    cross_chain_id,
                    source_address,
                    destination_address,
                    destination_chain,
                    &abi_payload,
                )?;

                let payload = GMPPayload::decode(&abi_payload)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;
                let (_, _instruction_accounts) = accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);

                match payload {
                    GMPPayload::InterchainTransfer(_interchain_token_transfer) => {
                        msg!("Received InterchainTransfer message");
                    }
                    GMPPayload::DeployInterchainToken(deploy_interchain_token) => {
                        msg!("Received DeployInterchainToken message");
                        msg!("Token ID: {:?}", deploy_interchain_token.token_id);
                    }
                    GMPPayload::DeployTokenManager(_deploy_token_manager) => {
                        msg!("Received DeployTokenManager message");
                    }
                }
            }
        }

        Ok(())
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    pda_bump: u8,
) -> ProgramResult {
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
    let data = state::InterchainTokenService { bump: pda_bump };

    program_utils::init_pda(
        payer,
        its_root_pda,
        program_id,
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
