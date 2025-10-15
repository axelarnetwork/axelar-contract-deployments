//! # Instruction Module
//!
//! This module provides constructors and definitions for all instructions that can be issued to the

use anchor_discriminators_macros::InstructionDiscriminator;
use solana_program::program_error::ProgramError;
use solana_program::system_program;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Top-level instructions supported by the Axelar Solana Gas Service program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, InstructionDiscriminator)]
pub enum GasServiceInstruction {
    /// Initialize the configuration PDA.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`payer`) paying for PDA creation
    /// 1. `[]` The `operator` account of this PDA.
    /// 2. `[writable]` The `config_pda` account to be created.
    /// 3. `[]` The `system_program` account.
    Initialize,

    /// Transfer operatorship of the gas service to a new operator.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The current `operator` account
    /// 1. `[]` The new `operator` account to transfer operatorship to
    /// 2. `[writable]` The `config_pda` account
    TransferOperatorship,

    /// Pay gas fees for a contract call using native SOL.
    ///
    /// Accounts expected:
    /// 0. `[signer, writable]` The account (`sender`) paying the gas fee in lamports.
    /// 1. `[writable]` The `config_pda` account that receives the lamports.
    /// 2. `[]` The `system_program` account.
    PayGas {
        /// The target blockchain for the contract call.
        destination_chain: String,
        /// The destination address on the target chain.
        destination_address: String,
        /// A 32-byte hash representing the payload.
        payload_hash: [u8; 32],
        /// The amount of SOL to pay as gas fees.
        amount: u64,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Add more native SOL gas to an existing transaction.
    ///
    /// Accounts expected:
    /// 1. `[signer, writable]` The account (`sender`) providing the additional lamports.
    /// 2. `[writable]` The `config_pda` account that receives the additional lamports.
    /// 3. `[]` The `system_program` account.
    AddGas {
        /// Message Id
        message_id: String,
        /// The additional SOL to add as gas.
        amount: u64,
        /// Where refunds should be sent.
        refund_address: Pubkey,
    },

    /// Collect accrued native SOL fees (operator only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `operator` account authorized to collect fees.
    /// 2. `[writable]` The `config_pda` account holding the accrued lamports to collect.
    /// 3. `[writable]` The `receiver` account where the collected lamports will be sent.
    CollectFees {
        /// The amount of SOL to collect as fees.
        amount: u64,
    },

    /// Refund previously collected native SOL fees (operator only).
    ///
    /// Accounts expected:
    /// 1. `[signer, read-only]` The `operator` account authorized to issue refunds.
    /// 2. `[writable]` The `receiver` account that will receive the refunded lamports.
    /// 3. `[writable]` The `config_pda` account from which lamports are refunded.
    RefundFees {
        /// Message Id
        message_id: String,
        /// The amount of SOL to be refunded.
        amount: u64,
    },
}

/// Builds an instruction to initialize the configuration PDA.
///
/// # Errors
/// - ix data cannot be serialized
pub fn init_config(payer: &Pubkey, operator: &Pubkey) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::Initialize)?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to transfer operatorship of the gas service.
///
/// # Errors
/// - if the instruction could not be serialized
pub fn transfer_operatorship(
    current_operator: &Pubkey,
    new_operator: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::TransferOperatorship)?;
    let (config_pda, _bump) = crate::get_config_pda();

    let accounts = vec![
        AccountMeta::new(*current_operator, true),
        AccountMeta::new_readonly(*new_operator, false),
        AccountMeta::new(config_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to pay native SOL for a contract call.
///
/// # Errors
/// - ix data cannot be serialized
#[allow(clippy::too_many_arguments)]
pub fn pay_gas_instruction(
    sender: &Pubkey,
    destination_chain: String,
    destination_address: String,
    payload_hash: [u8; 32],
    refund_address: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::PayGas {
        destination_chain,
        destination_address,
        payload_hash,
        refund_address,
        amount,
    })?;
    let (config_pda, _bump) = crate::get_config_pda();

    let (event_authority, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction to add native SOL gas.
///
/// # Errors
/// - ix data cannot be serialized
pub fn add_gas_instruction(
    sender: &Pubkey,
    message_id: String,
    amount: u64,
    refund_address: Pubkey,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::AddGas {
        message_id,
        amount,
        refund_address,
    })?;
    let (config_pda, _bump) = crate::get_config_pda();

    let (event_authority, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to collect native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn collect_fees_instruction(
    operator: &Pubkey,
    receiver: &Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::CollectFees { amount })?;
    let (config_pda, _bump) = crate::get_config_pda();

    let (event_authority, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}

/// Builds an instruction for the operator to refund previously collected native SOL fees.
///
/// # Errors
/// - ix data cannot be serialized
pub fn refund_fees_instruction(
    operator: &Pubkey,
    receiver: &Pubkey,
    message_id: String,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let ix_data = borsh::to_vec(&GasServiceInstruction::RefundFees { message_id, amount })?;
    let (config_pda, _) = crate::get_config_pda();

    let (event_authority, _bump) =
        Pubkey::find_program_address(&[event_cpi::EVENT_AUTHORITY_SEED], &crate::ID);

    let accounts = vec![
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new(*receiver, false),
        AccountMeta::new(config_pda, false),
        AccountMeta::new_readonly(event_authority, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data: ix_data,
    })
}
