//! Program state processor.

use borsh::from_slice;
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

mod call_contract;
mod execute;
mod initialize_approved_message;
mod initialize_config;
mod initialize_execute_data;
mod initialize_transfer_operatorship;
mod transfer_operatorship;
mod validate_contract_call;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = from_slice::<GatewayInstruction>(input)?;
        check_program_account(*program_id)?;

        match instruction {
            GatewayInstruction::Execute {} => {
                msg!("Instruction: Execute");
                Self::process_execute(program_id, accounts)
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
                Self::process_initialize_execute_data(program_id, accounts, &execute_data)
            }
            GatewayInstruction::TransferOperatorship {} => {
                msg!("Instruction: TransferOperatorship");
                Self::process_transfer_operatorship(program_id, accounts)
            }
            GatewayInstruction::InitializeMessage {
                command_id: message_id,
                source_chain,
                source_address,
                payload_hash,
                destination_program,
            } => {
                msg!("Instruction: Initialize Approved Message");
                Self::process_initialize_approved_message(
                    program_id,
                    accounts,
                    message_id,
                    source_chain,
                    source_address,
                    payload_hash,
                    destination_program,
                )
            }
            GatewayInstruction::InitializeTransferOperatorship {
                operators_and_weights,
                threshold,
            } => {
                msg!("Instruction: Initialize TransferOperatorship");
                Self::process_initialize_transfer_operatorship(
                    program_id,
                    accounts,
                    operators_and_weights,
                    threshold,
                )
            }
            GatewayInstruction::ValidateContractCall {
                destination_program,
                command_id: message_id,
                payload_hash,
                source_address,
                source_chain,
            } => {
                msg!("Instruction: Validate Contract Call");
                Self::process_validate_contract_call(
                    program_id,
                    message_id,
                    source_chain,
                    source_address,
                    payload_hash,
                    destination_program,
                    accounts,
                )
            }
        }
    }
}

/// Initialize a Gateway PDA
fn init_pda<'a, 'b, T: borsh::BorshSerialize>(
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

mod helper {
    use solana_program::account_info::AccountInfo;
    use solana_program::pubkey::Pubkey;

    use crate::error::GatewayError;

    /// Compares the account address with the expected address.
    pub(super) fn compare_address(
        pda_info: &AccountInfo<'_>,
        expected_pda_info: Pubkey,
    ) -> Result<(), GatewayError> {
        if pda_info.key != &expected_pda_info {
            Err(GatewayError::IncorrectAccountAddr)
        } else {
            Ok(())
        }
    }
}
