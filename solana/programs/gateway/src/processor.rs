//! Program state processor

use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::events::emit_call_contract_event;
use crate::instruction::GatewayInstruction;
use crate::{check_program_account, find_root_pda};

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = GatewayInstruction::unpack(input)?;
        match instruction {
            GatewayInstruction::Queue { .. } => {
                msg!("Instruction: Queue");
            }
            GatewayInstruction::CallContract {
                sender,
                destination_chain,
                destination_contract_address,
                payload,
                payload_hash,
            } => {
                msg!("Instruction: CallContract");
                emit_call_contract_event(
                    &sender,
                    destination_chain,
                    destination_contract_address,
                    payload,
                    &payload_hash,
                )?
            }
            GatewayInstruction::Initialize { payload } => {
                msg!("Instruction: Initialize");
                msg!("D: {:?}", payload);
                initialize(program_id, accounts, payload)?;
            }
        };
        Ok(())
    }
}

/// Initialize Gateway root PDA.
pub(crate) fn initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> Result<(), ProgramError> {
    check_program_account(*program_id)?;

    let accounts_iter = &mut accounts.iter();

    let payer_info = next_account_info(accounts_iter)?;
    let pda_info = next_account_info(accounts_iter)?;
    let system_program_info = next_account_info(accounts_iter)?;

    let (expected_pda_info, bump) = find_root_pda();

    assert_eq!(pda_info.key, &expected_pda_info);
    assert_eq!(pda_info.lamports(), 0);

    let rent = Rent::get()?;
    let ix = &system_instruction::create_account(
        payer_info.key,
        pda_info.key,
        rent.minimum_balance(data.len().max(1)),
        data.len() as u64,
        &crate::id(),
    );
    invoke_signed(
        ix,
        &[
            payer_info.clone(),
            pda_info.clone(),
            system_program_info.clone(),
        ],
        &[&[&[bump]]],
    )?;

    let mut account_data = pda_info.try_borrow_mut_data()?;
    account_data[..data.len()].copy_from_slice(data);

    Ok(())
}
