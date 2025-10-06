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
    spl::{add_spl_gas, collect_fees_spl, process_pay_spl_for_contract_call, refund_spl},
    transfer_operatorship::process_transfer_operatorship,
};

mod initialize;
mod native;
mod spl;
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
        // Spl token instructions
        GasServiceInstruction::PaySplForContractCall {
            destination_chain,
            destination_address,
            payload_hash,
            gas_fee_amount,
            decimals,
            refund_address,
        } => process_pay_spl_for_contract_call(
            program_id,
            accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
            decimals,
        ),
        GasServiceInstruction::AddSplGas {
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            decimals,
            refund_address,
        } => add_spl_gas(
            program_id,
            accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            refund_address,
            decimals,
        ),
        GasServiceInstruction::CollectSplFees { amount, decimals } => {
            collect_fees_spl(program_id, accounts, amount, decimals)
        }
        GasServiceInstruction::RefundSplFees {
            tx_hash,
            ix_index,
            event_ix_index,
            fees,
            decimals,
        } => refund_spl(
            program_id,
            accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            fees,
            decimals,
        ),

        // Native token instructions
        GasServiceInstruction::PayNativeForContractCall {
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
        } => process_pay_native_for_contract_call(
            program_id,
            accounts,
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            gas_fee_amount,
        ),
        GasServiceInstruction::AddNativeGas {
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            refund_address,
        } => add_native_gas(
            program_id,
            accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            gas_fee_amount,
            refund_address,
        ),
        GasServiceInstruction::CollectNativeFees { amount } => {
            collect_fees_native(program_id, accounts, amount)
        }
        GasServiceInstruction::RefundNativeFees {
            tx_hash,
            ix_index,
            event_ix_index,
            fees,
        } => refund_native(
            program_id,
            accounts,
            tx_hash,
            ix_index,
            event_ix_index,
            fees,
        ),
    }
}
