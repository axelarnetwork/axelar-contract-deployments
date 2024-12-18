//! Processor for the Solana gas service

use borsh::BorshDeserialize;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    check_program_account,
    instructions::{GasServiceInstruction, PayWithNativeToken, PayWithSplToken},
};

pub use self::native::{
    NativeGasAddedEvent, NativeGasPaidForContractCallEvent, NativeGasRefundedEvent,
};
pub use self::spl::{SplGasAddedEvent, SplGasPaidForContractCallEvent, SplGasRefundedEvent};
use self::{
    initialize::process_initialize_config,
    native::{
        add_native_gas, collect_fees_native, process_pay_native_for_contract_call, refund_native,
    },
    spl::{add_spl_gas, collect_fees_spl, process_pay_spl_for_contract_call, refund_spl},
};

mod initialize;
mod native;
mod spl;

/// Processes an instruction.
///
/// # Errors
/// - if the ix processing resulted in an error
#[allow(clippy::todo)]
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    input: &[u8],
) -> ProgramResult {
    let instruction = GasServiceInstruction::try_from_slice(input)?;
    check_program_account(*program_id)?;

    match instruction {
        GasServiceInstruction::Initialize { salt } => {
            process_initialize_config(program_id, accounts, salt)
        }
        GasServiceInstruction::SplToken(ix) => match ix {
            PayWithSplToken::ForContractCall {
                destination_chain,
                destination_address,
                payload_hash,
                gas_fee_amount,
                params,
                decimals,
                refund_address,
            } => process_pay_spl_for_contract_call(
                program_id,
                accounts,
                destination_chain,
                destination_address,
                payload_hash,
                refund_address,
                &params,
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
                params,
                gas_fee_amount,
            } => process_pay_native_for_contract_call(
                program_id,
                accounts,
                destination_chain,
                destination_address,
                payload_hash,
                refund_address,
                &params,
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

/// Even emitted by the Axelar Solana Gas service
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum GasServiceEvent {
    /// Event when SOL was used to pay for a contract call
    NativeGasPaidForContractCall(NativeGasPaidForContractCallEvent),
    /// Event when SOL was added to fund an already emitted contract call
    NativeGasAdded(NativeGasAddedEvent),
    /// Event when SOL was refunded
    NativeGasRefunded(NativeGasRefundedEvent),
    /// Event when an SPL token was used to pay for a contract call
    SplGasPaidForContractCall(SplGasPaidForContractCallEvent),
    /// Event when an SPL token was added to fund an already emitted contract call
    SplGasAdded(SplGasAddedEvent),
    /// Event when an SPL token was refunded
    SplGasRefunded(SplGasRefundedEvent),
}
