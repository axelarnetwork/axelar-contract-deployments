//! # Instruction Module
//!
//! This module provides constructors and definitions for all instructions that can be issued to the

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::system_program;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Top-level instructions supported by the Axelar Solana Gas Service program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum GasServiceInstruction {
    /// Initialize the configuration PDA.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`payer`) paying for PDA creation
    /// 1. `[]` The `authority` account of this PDA.
    /// 1. `[writable]` The `config_pda` account to be created.
    /// 2. `[]` The `system_program` account.
    Initialize {
        /// A unique 32-byte array used as a seed in deriving the config PDA.
        salt: [u8; 32],
    },

    /// Use SPL tokens to pay for gas-related operations.
    SplToken(PayWithSplToken),

    /// Use native SOL to pay for gas-related operations.
    Native(PayWithNativeToken),
}

/// Instructions related to paying gas fees with SPL tokens.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum PayWithSplToken {
    /// Pay gas fees for a contract call using SPL tokens.
    ForContractCall {
        /// The account paying the gas fee.
        sender: Pubkey,
        /// The target blockchain (e.g., "ethereum") for the contract call.
        destination_chain: String,
        /// The recipient address on the destination chain.
        destination_address: String,
        /// A 32-byte hash representing the payload.
        payload_hash: [u8; 32],
        /// The SPL token mint used for the gas fee.
        gas_token: Pubkey,
        /// The amount of tokens to be paid as gas fees.
        gas_fee_amount: u64,
        /// Where refunds should be sent
        refund_address: Pubkey,
    },

    /// Add more gas (SPL tokens) to an existing contract call.
    AddGas {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The account paying the additional gas fee.
        pubkey: Pubkey,
        /// The additional SPL tokens to add as gas.
        gas_fee_amount: u64,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Collect fees that have accrued in SPL tokens (authority only).
    CollectFees {
        /// The amount of SPL tokens to be collected as fees.
        amount: u64,
    },

    /// Refund previously collected SPL token fees (authority only).
    Refund {
        /// A 64-byte unique transaction identifier
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction
        log_index: u64,
        /// The amount of SPL tokens to be refunded
        fees: u64,
    },
}

/// Instructions related to paying gas fees with native SOL.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize)]
pub enum PayWithNativeToken {
    /// Pay gas fees for a contract call using native SOL.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`payer`) paying the gas fee in lamports.
    /// 1. `[writable]` The `config_pda` account that receives the lamports.
    /// 2. `[]` The `system_program` account.
    ForContractCall {
        /// The target blockchain for the contract call.
        destination_chain: String,
        /// The destination address on the target chain.
        destination_address: String,
        /// A 32-byte hash representing the payload.
        payload_hash: [u8; 32],
        /// Where refunds should be sent.
        refund_address: Pubkey,
        /// Additional parameters for the contract call.
        params: Vec<u8>,
        /// The amount of SOL to pay as gas fees.
        gas_fee_amount: u64,
    },

    /// Add more native SOL gas to an existing transaction.
    ///
    /// Accounts expected:
    /// 1. `[signer, writable]` The account (`sender`) providing the additional lamports.
    /// 2. `[writable]` The `config_pda` account that receives the additional lamports.
    /// 3. `[]` The `system_program` account.
    AddGas {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The additional SOL to add as gas.
        gas_fee_amount: u64,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Collect accrued native SOL fees (authority only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `authority` account authorized to collect fees.
    /// 2. `[writable]` The `config_pda` account holding the accrued lamports to collect.
    /// 3. `[writable]` The `receiver` account where the collected lamports will be sent.
    CollectFees {
        /// The amount of SOL to collect as fees.
        amount: u64,
    },

    /// Refund previously collected native SOL fees (authority only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `authority` account authorized to issue refunds.
    /// 2. `[writable]` The `receiver` account that will receive the refunded lamports.
    /// 3. `[writable]` The `config_pda` account from which lamports are refunded.
    Refund {
        /// A 64-byte unique transaction identifier.
        tx_hash: [u8; 64],
        /// The index of the log entry in the transaction.
        log_index: u64,
        /// The amount of SOL to be refunded.
        fees: u64,
    },
}

/// Builds an instruction to initialize the configuration PDA.
///
/// # Errors
/// - ix data cannot be serialized
pub fn init_config(
    program_id: &Pubkey,
    payer: &Pubkey,
    authority: &Pubkey,
    config_pda: &Pubkey,
    salt: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Initialize { salt })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*authority, false),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to pay native SOL for a contract call.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn pay_native_for_contract_call_instruction(
    program_id: &Pubkey,
    payer: &Pubkey,
    config_pda: &Pubkey,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    params: Vec<u8>,
    gas_fee_amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(
        PayWithNativeToken::ForContractCall {
            destination_chain,
            destination_address,
            payload_hash,
            refund_address,
            params,
            gas_fee_amount,
        },
    ))?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to add native SOL gas.
///
/// # Errors
/// - ix data cannot be serialized
pub fn add_native_gas_instruction(
    program_id: &Pubkey,
    sender: &Pubkey,
    config_pda: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    gas_fee_amount: u64,
    refund_address: Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(PayWithNativeToken::AddGas {
        tx_hash,
        log_index,
        gas_fee_amount,
        refund_address,
    }))?;

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the authority to collect native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn collect_native_fees_instruction(
    program_id: &Pubkey,
    authority: &Pubkey,
    config_pda: &Pubkey,
    receiver: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(
        PayWithNativeToken::CollectFees { amount },
    ))?;

    let accounts = vec![
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(*config_pda, false),
        AccountMeta::new(*receiver, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the authority to refund previously collected native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn refund_native_fees_instruction(
    program_id: &Pubkey,
    authority: &Pubkey,
    receiver: &Pubkey,
    config_pda: &Pubkey,
    tx_hash: [u8; 64],
    log_index: u64,
    fees: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Native(PayWithNativeToken::Refund {
        tx_hash,
        log_index,
        fees,
    }))?;

    let accounts = vec![
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new(*config_pda, false),
    ];

    Ok(Instruction {
        program_id: *program_id,
        accounts,
        data: ix_data,
    })
}
