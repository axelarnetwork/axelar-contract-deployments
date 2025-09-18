//! Processor for the Solana gas service

use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    check_program_account,
    instructions::{GasServiceInstruction, PayWithNativeToken, PayWithSplToken},
};

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
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    let instruction = GasServiceInstruction::try_from_slice(input)?;
    check_program_account(*program_id)?;

    match instruction {
        GasServiceInstruction::Initialize => process_initialize_config(program_id, accounts),
        GasServiceInstruction::TransferOperatorship => {
            process_transfer_operatorship(program_id, accounts)
        }
        GasServiceInstruction::SplToken(ix) => match ix {
            PayWithSplToken::ForContractCall {
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
            PayWithSplToken::AddGas {
                tx_hash,
                log_index,
                gas_fee_amount,
                decimals,
                refund_address,
            } => add_spl_gas(
                program_id,
                accounts,
                tx_hash,
                log_index,
                gas_fee_amount,
                refund_address,
                decimals,
            ),
            PayWithSplToken::CollectFees { amount, decimals } => {
                collect_fees_spl(program_id, accounts, amount, decimals)
            }
            PayWithSplToken::Refund {
                tx_hash,
                log_index,
                fees,
                decimals,
            } => refund_spl(program_id, accounts, tx_hash, log_index, fees, decimals),
        },
        GasServiceInstruction::Native(ix) => match ix {
            PayWithNativeToken::ForContractCall {
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
            PayWithNativeToken::AddGas {
                tx_hash,
                log_index,
                gas_fee_amount,
                refund_address,
            } => add_native_gas(
                program_id,
                accounts,
                tx_hash,
                log_index,
                gas_fee_amount,
                refund_address,
            ),
            PayWithNativeToken::CollectFees { amount } => {
                collect_fees_native(program_id, accounts, amount)
            }
            PayWithNativeToken::Refund {
                tx_hash,
                log_index,
                fees,
            } => refund_native(program_id, accounts, tx_hash, log_index, fees),
        },
    }
}
