//! Instruction module; consist of fasade instructions, test ix constructors and
//! internal helpers.

use axelar_message_primitives::U256;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::{LogIndex, TxHash};

/// Instructions supported by the program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum GasServiceInstruction {
    /// Instruction to initialize the program root pda.
    Initialize,

    /// Instruction PayNativeGasForContractCall.
    PayNativeGasForContractCall {
        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload] Data payload for the contract call.
        payload: Vec<u8>,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction PayNativeGasForContractCallWithToken.
    PayNativeGasForContractCallWithToken {
        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload] Data payload for the contract call.
        payload: Vec<u8>,

        /// [symbol] The symbol of the token used to pay for gas.
        symbol: Vec<u8>,

        /// [amount] The amount of tokens to pay for gas.
        amount: U256,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction PayNativeGasForExpressCall.
    PayNativeGasForExpressCall {
        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload] Data payload for the contract call.
        payload: Vec<u8>,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction PayNativeGasForExpressCallWithToken.
    PayNativeGasForExpressCallWithToken {
        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload] Data payload for the contract call.
        payload: Vec<u8>,

        /// [symbol] The symbol of the token.
        symbol: Vec<u8>,

        /// [amount] The amount of tokens.
        amount: U256,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction AddNativeGas.
    AddNativeGas {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction AddNativeExpressGas.
    AddNativeExpressGas {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Instruction CollectFees
    /// Authority only.
    CollectFees {
        /// [amount] The amount of SOL to send.
        amount: u64,
    },

    /// Instruction Refund
    /// Authority only.
    Refund {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,
    },
}

/// Creates a [`Initialize`] instruction.
///
/// [initializer] The address of the initializer / payer / admin / collector.
pub fn create_initialize_root_pda_ix(initializer: Pubkey) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::Initialize)?;

    let accounts = vec![
        AccountMeta::new(initializer, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`PayNativeGasForContractCall`] instruction.
///
/// [sender] The address making the payment.
///
/// [refund_address] The address where refunds, if any, should be sent.
///
/// [destination_chain] The target chain.
///
/// [destination_address] The target contract address on the destination chain.
///
/// [payload] Data payload for the contract call.
///
/// [fees] The amount of SOL to pay for gas.
pub fn create_pay_native_gas_for_contract_call_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::PayNativeGasForContractCall {
        destination_chain,
        destination_address,
        payload,
        fees,
        refund_address,
    })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`PayNativeGasForContractCallWithToken`] instruction.
///
/// [sender] The address making the payment.
///
/// [refund_address] The address where refunds, if any, should be sent.
///
/// [destination_chain] The target chain.
///
/// [destination_address] The target contract address on the destination chain.
///
/// [payload] Data payload for the contract call.
///
/// [symbol] The symbol of the token used to pay for gas.
///
/// [amount] The amount of tokens to pay for gas.
///
/// [fees] The amount of SOL to pay for gas.
#[allow(clippy::too_many_arguments)]
pub fn create_pay_native_gas_for_contract_call_with_token_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    symbol: Vec<u8>,
    amount: U256,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(
        &GasServiceInstruction::PayNativeGasForContractCallWithToken {
            destination_chain,
            destination_address,
            payload,
            symbol,
            amount,
            fees,
            refund_address,
        },
    )?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`PayNativeGasForExpressCall`] instruction.
///
/// [sender] The address making the payment.
///
/// [refund_address] The address where refunds, if any, should be sent.
///
/// [destination_chain] The target chain.
///
/// [destination_address] The target contract address on the destination chain.
///
/// [payload] Data payload for the contract call.
///
/// [fees] The amount of SOL to pay for gas.
pub fn create_pay_native_gas_for_express_call_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::PayNativeGasForExpressCall {
        destination_chain,
        destination_address,
        payload,
        fees,
        refund_address,
    })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`PayNativeGasForExpressCallWithToken`] instruction.
///
/// [sender] The address making the payment.
///
/// [refund_address] The address where refunds, if any, should be sent.
///
/// [destination_chain] The target chain.
///
/// [destination_address] The target contract address on the destination chain.
///
/// [payload] Data payload for the contract call.
///
/// [symbol] The symbol of the token.
///
/// [amount] The amount of tokens.
///
/// [fees] The amount of SOL to pay for gas.
#[allow(clippy::too_many_arguments)]
pub fn create_pay_native_gas_for_express_call_with_token_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    symbol: Vec<u8>,
    amount: U256,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(
        &GasServiceInstruction::PayNativeGasForExpressCallWithToken {
            destination_chain,
            destination_address,
            payload,
            symbol,
            amount,
            fees,
            refund_address,
        },
    )?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`AddNativeGas`] instruction.
///
/// [tx_hash] The hash of the transaction on the destination chain.
///
/// [log_index] The log index of the event.
///
/// [fees] The amount of SOL to pay for gas.
///
/// [refund_address] The address where refunds, if any, should be sent.
pub fn create_add_native_gas_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    tx_hash: TxHash,
    log_index: LogIndex,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::AddNativeGas {
        tx_hash,
        log_index,
        fees,
        refund_address,
    })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`AddNativeExpressGas`] instruction.
///
/// [tx_hash] The hash of the transaction on the destination chain.
///
/// [log_index] The log index of the event.
///
/// [fees] The amount of SOL to pay for gas.
///
/// [refund_address] The address where refunds, if any, should be sent.
pub fn create_add_native_express_gas_ix(
    sender: Pubkey,
    refund_address: Pubkey,
    tx_hash: TxHash,
    log_index: LogIndex,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::AddNativeExpressGas {
        tx_hash,
        log_index,
        fees,
        refund_address,
    })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`CollectFees`] instruction. Authority only.
///
/// [receiver] The address where SOL should be sent.
///
/// [amount] The amount of SOL to send.
pub fn create_collect_fees_ix(
    sender: Pubkey,
    receiver: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::CollectFees { amount })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new(receiver, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`Refund`] instruction. Authority only.
///
/// [receiver] The address where SOL should be sent.
///
/// [amount] The amount of SOL to send.
pub fn create_refund_ix(
    sender: Pubkey,
    receiver: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GasServiceInstruction::CollectFees { amount })?;

    let accounts = vec![
        AccountMeta::new(sender, true),
        AccountMeta::new(crate::get_gas_service_root_pda().0, false),
        AccountMeta::new(receiver, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
