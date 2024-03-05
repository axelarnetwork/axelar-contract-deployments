//! Program state processor

mod deploy_remote_interchain_token;
mod deploy_remote_token_manager;
mod execute;
mod give_token;
mod initialize;
mod remote_interchain_transfer;
mod take_token;

use borsh::BorshDeserialize;
use program_utils::check_program_account;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::InterchainTokenServiceInstruction;
use crate::state::RootPDA;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        check_program_account(program_id, crate::check_id)?;

        let instruction = InterchainTokenServiceInstruction::try_from_slice(input)?;

        match instruction {
            InterchainTokenServiceInstruction::Execute { payload } => {
                msg!("Instruction: Execute");
                Self::execute(program_id, accounts, payload)
            }
            InterchainTokenServiceInstruction::Initialize {} => {
                msg!("Instruction: Initialize");
                Self::process_initialize(program_id, accounts)
            }
            InterchainTokenServiceInstruction::GiveToken {
                token_manager_type,
                amount,
            } => {
                msg!("Instruction: GiveToken");
                Self::give_token(program_id, accounts, token_manager_type, amount)
            }
            InterchainTokenServiceInstruction::TakeToken {
                token_manager_type,
                amount,
            } => {
                msg!("Instruction: TakeToken");
                Self::take_token(program_id, accounts, token_manager_type, amount)
            }
            InterchainTokenServiceInstruction::DeployRemoteTokenManager {
                salt,
                destination_chain,
                token_manager_type,
                params,
                gas_value,
            } => {
                msg!("Instruction: DeployRemoteTokenManager");
                Self::deploy_remote_token_manager(
                    program_id,
                    accounts,
                    salt,
                    destination_chain,
                    token_manager_type,
                    params,
                    gas_value,
                )
            }
            InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
                salt,
                destination_chain,
                name,
                symbol,
                decimals,
                minter,
                gas_value,
            } => {
                msg!("Instruction: DeployRemoteInterchainToken");
                Self::deploy_remote_interchain_token(
                    program_id,
                    accounts,
                    salt,
                    destination_chain,
                    name,
                    symbol,
                    decimals,
                    minter,
                    gas_value,
                )
            }
            InterchainTokenServiceInstruction::RemoteInterchainTransfer {
                token_id,
                destination_chain,
                destination_address,
                amount,
                data,
                metadata_version,
                symbol,
                token_manager_type,
            } => {
                msg!("Instruction: RemoteInterchainTransfer");
                Self::remote_interchain_transfer(
                    program_id,
                    accounts,
                    token_id,
                    destination_chain,
                    destination_address,
                    amount,
                    data,
                    metadata_version,
                    symbol,
                    token_manager_type,
                )
            }
        }
    }
}

pub(crate) fn assert_root_its_derivation(
    gateway_root_pda: &AccountInfo<'_>,
    gas_service_root_pda: &AccountInfo<'_>,
    root_pda: &RootPDA,
    its_root_pda: &AccountInfo<'_>,
) -> Result<(), ProgramError> {
    let actual_root_pda = Pubkey::create_program_address(
        &[
            &gateway_root_pda.key.as_ref(),
            &gas_service_root_pda.key.as_ref(),
            &[root_pda.bump_seed],
        ],
        &crate::id(),
    )?;
    if actual_root_pda != *its_root_pda.key {
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}
