//! Processor for the Solana gas service

use borsh::BorshDeserialize;
use event_cpi_macros::event_cpi_handler;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{check_program_account, instructions::GasServiceInstruction};

use self::{
    initialize::process_initialize_config,
    native::{
        add_native_gas, collect_fees_native, process_pay_native_for_contract_call, refund_native,
    },
    transfer_operatorship::process_transfer_operatorship,
};

mod initialize;
mod native;
mod transfer_operatorship;

/// Processes an instruction.
///
/// # Errors
/// - if the ix processing resulted in an error
#[allow(clippy::too_many_lines)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    check_program_account(*program_id)?;

    event_cpi_handler!(input);

    let instruction = GasServiceInstruction::try_from_slice(input)?;

    match instruction {
        GasServiceInstruction::Initialize => process_initialize_config(program_id, accounts),
        GasServiceInstruction::TransferOperatorship => {
            process_transfer_operatorship(program_id, accounts)
        }

        // Native token instructions
        GasServiceInstruction::PayGas {
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            amount,
        } => process_pay_native_for_contract_call(
            program_id,
            accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            amount,
        ),

        GasServiceInstruction::AddGas {
            message_id,
            amount,
            refund_address,
        } => add_native_gas(program_id, accounts, message_id, amount, refund_address),

        GasServiceInstruction::CollectFees { amount } => {
            collect_fees_native(program_id, accounts, amount)
        }

        GasServiceInstruction::RefundFees { message_id, amount } => {
            refund_native(program_id, accounts, message_id, amount)
        }
    }
}
