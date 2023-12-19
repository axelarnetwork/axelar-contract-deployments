//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::id;

/// Instructions supported by the OperatorInstruction program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum OperatorInstruction {
    /// Initialize a new set of Operators
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the setup of new
    ///      operator chain creation
    ///   1. `[writable]` The new operator group account that needs to be
    ///      created
    ///   2. `[writable]` The operator account that needs to be created, the
    ///      first operator in the group
    ///   3. `[signer]` The initial operator
    ///   4. `[]` The system program
    CreateOperatorGroup {
        /// Unique identifier the the operator chain
        id: String,
    },
    /// Sets the trusted address and its hash for a remote chain.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the account creation
    ///   1. `[]` The operator group account
    ///   2. `[]` The EXISTING operator account of the current operator
    ///   3. `[signer]` The owner of the EXISTING operator account
    ///   4. `[]` The owner of the NEW operator account
    ///   5. `[writable]` The NEW operator account for the new operator
    ///   6. `[]` The system program
    AddOperator,
}

/// Create `Creategroup` instruction
pub fn build_create_group_instruction(
    funder: &Pubkey,
    operator_group_pda: &Pubkey,
    operator_pda: &Pubkey,
    operator: &Pubkey,
    op_id: String,
) -> Result<Instruction, ProgramError> {
    let init_data = OperatorInstruction::CreateOperatorGroup { id: op_id };
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*operator_group_pda, false),
        AccountMeta::new(*operator_pda, false),
        AccountMeta::new_readonly(*operator, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}

/// Create `AddOperator` instruction
pub fn build_add_operator_instruction(
    funder: &Pubkey,
    existing_operator_group_pda_acc: &Pubkey,
    existing_operator_pda_acc: &Pubkey,
    existing_operator_acc_owner: &Pubkey,
    new_operator_acc_owner: &Pubkey,
    new_operator_pda_acc: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let init_data = OperatorInstruction::AddOperator;
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new_readonly(*existing_operator_group_pda_acc, false),
        AccountMeta::new_readonly(*existing_operator_pda_acc, false),
        AccountMeta::new_readonly(*existing_operator_acc_owner, true),
        AccountMeta::new_readonly(*new_operator_acc_owner, false),
        AccountMeta::new(*new_operator_pda_acc, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}
