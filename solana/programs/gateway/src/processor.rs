//! Program state processor.

use borsh::from_slice;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction, system_program};

use crate::accounts::{GatewayConfig, GatewayExecuteData, GatewayMessageID};
use crate::check_program_account;
use crate::error::GatewayError;
use crate::events::emit_call_contract_event;
use crate::instructions::transfer_op::transfer_operatorship;
use crate::instructions::GatewayInstruction;

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
                Self::execute(accounts)
            }
            GatewayInstruction::CallContract {
                sender,
                destination_chain,
                destination_contract_address,
                payload,
                payload_hash,
            } => {
                msg!("Instruction: Call Contract");
                emit_call_contract_event(
                    &sender,
                    destination_chain.as_bytes(),
                    &destination_contract_address,
                    &payload,
                    &payload_hash,
                )?;
                Ok(())
            }
            GatewayInstruction::InitializeConfig { config } => {
                msg!("Instruction: Initialize Config");
                Self::initialize_config(accounts, &config)
            }
            GatewayInstruction::InitializeExecuteData { execute_data } => {
                msg!("Instruction: Initialize Execute Data");
                Self::initialize_execute_data(accounts, &execute_data)
            }
            GatewayInstruction::TransferOperatorship {} => {
                msg!("Instruction: TransferOperatorship");
                transfer_operatorship(program_id, accounts)
            }
            GatewayInstruction::InitializeMessage { message_id } => {
                msg!("Instruction: Initialize Message ID");
                Self::initialize_message_id(accounts, &message_id)
            }
        }
    }

    /// Execute the payload.
    pub fn execute(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        // FIXME: implement the actual instruction here

        // DEBUG: print all accounts
        let mut accounts = accounts.iter();
        while let Ok(account) = next_account_info(&mut accounts) {
            msg!("Instruction Account: {:#?}", account);
        }
        Ok(())
    }

    /// Initialize Gateway Config account.
    pub fn initialize_config(
        accounts: &[AccountInfo],
        gateway_config: &GatewayConfig,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let gateway_config_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Gateway Config account uses the canonical bump.
        let (canonical_pda, canonical_bump) = crate::find_root_pda();
        if *gateway_config_account.key != canonical_pda {
            return Err(GatewayError::InvalidConfigAccount.into());
        }

        init_pda(
            payer,
            gateway_config_account,
            &[&[canonical_bump]],
            gateway_config,
        )
    }

    fn initialize_execute_data(
        accounts: &[AccountInfo<'_>],
        execute_data: &GatewayExecuteData,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let payer = next_account_info(accounts_iter)?;
        let execute_data_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Execute Data account uses the canonical bump.
        let (canonical_pda, bump, seeds) = execute_data.pda();
        if *execute_data_account.key != canonical_pda {
            return Err(GatewayError::InvalidExecuteDataAccount.into());
        }
        init_pda(
            payer,
            execute_data_account,
            &[seeds.as_ref(), &[bump]],
            execute_data,
        )
    }

    fn initialize_message_id(
        accounts: &[AccountInfo<'_>],
        message_id: &GatewayMessageID,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        let payer = next_account_info(accounts_iter)?;
        let message_id_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account
        if !system_program::check_id(system_account.key) {
            return Err(GatewayError::InvalidSystemAccount.into());
        }

        // Check: Message ID account uses the canonical bump.
        let (canonical_pda, bump, seeds) = message_id.pda();
        if *message_id_account.key != canonical_pda {
            return Err(GatewayError::InvalidMessageIDAccount.into());
        }
        init_pda(
            payer,
            message_id_account,
            &[seeds.as_ref(), &[bump]],
            message_id,
        )
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
