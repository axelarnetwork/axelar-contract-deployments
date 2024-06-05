//! Program state processor.

use borsh::BorshDeserialize;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};

use crate::check_program_account;
use crate::instructions::GatewayInstruction;

mod approve_messages;
mod call_contract;
mod initialize_command;
mod initialize_config;
mod initialize_execute_data;
mod rotate_signers;
mod transfer_operatorship;
mod validate_message;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = GatewayInstruction::try_from_slice(input)?;
        check_program_account(*program_id)?;

        match instruction {
            GatewayInstruction::ApproveMessages {} => {
                msg!("Instruction: Approve Messages");
                Self::process_approve_messages(program_id, accounts)
            }
            GatewayInstruction::RotateSigners {} => {
                msg!("Instruction: Rotate Signers");
                Self::process_rotate_signers(program_id, accounts)
            }
            GatewayInstruction::CallContract {
                destination_chain,
                destination_contract_address,
                payload,
            } => {
                msg!("Instruction: Call Contract");
                Self::process_call_contract(
                    program_id,
                    accounts,
                    destination_chain,
                    destination_contract_address,
                    payload,
                )
            }
            GatewayInstruction::InitializeConfig { config } => {
                msg!("Instruction: Initialize Config");
                Self::process_initialize_config(program_id, accounts, config)
            }
            GatewayInstruction::InitializeExecuteData { execute_data } => {
                msg!("Instruction: Initialize Execute Data");
                Self::process_initialize_execute_data(program_id, accounts, execute_data)
            }
            GatewayInstruction::InitializePendingCommand(command) => {
                msg!("Instruction: Initialize Pending Command");
                Self::process_initialize_command(program_id, accounts, command)
            }
            GatewayInstruction::ValidateMessage(command) => {
                msg!("Instruction: Validate Message");
                Self::process_validate_message(program_id, accounts, command)
            }
            GatewayInstruction::TransferOperatorship => {
                msg!("Instruction: Transfer Operatorship");
                Self::process_transfer_operatorship(program_id, accounts)
            }
        }
    }
}

/// Initialize a Gateway PDA
fn init_pda_with_dynamic_size<'a, 'b, T: borsh::BorshSerialize>(
    payer: &'a AccountInfo<'b>,
    new_account_pda: &'a AccountInfo<'b>,
    seeds: &[&[u8]],
    data: &T,
) -> Result<(), ProgramError> {
    let serialized_data = borsh::to_vec(data)?;
    let space = serialized_data.len();
    let rent_sysvar = Rent::get()?;
    let rent = rent_sysvar.minimum_balance(space);

    assert!(payer.is_signer);
    assert!(payer.is_writable);
    // Note that `new_account_pda` is not a signer yet.
    // This program will sign for it via `invoke_signed`.
    assert!(!new_account_pda.is_signer);
    assert!(new_account_pda.is_writable);
    assert_eq!(new_account_pda.owner, &system_program::ID);

    invoke_signed(
        &system_instruction::create_account(
            payer.key,
            new_account_pda.key,
            rent,
            space
                .try_into()
                .map_err(|_| ProgramError::ArithmeticOverflow)?,
            &crate::ID,
        ),
        &[payer.clone(), new_account_pda.clone()],
        &[seeds],
    )?;
    let mut account_data = new_account_pda.try_borrow_mut_data()?;
    account_data[..space].copy_from_slice(&serialized_data);
    Ok(())
}
