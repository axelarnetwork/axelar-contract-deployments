//! Instruction types

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::hash::hash;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::id;

/// Unique identifier for a permission group
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct GroupId([u8; 32]);

/// Instructions supported by the OperatorInstruction program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum PermissionGroupInstruction {
    /// Initialize a new set of permissioned accounts.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the setup of new
    ///      permission group
    ///   1. `[writable]` The new permission group account that needs to be
    ///      created
    ///   2. `[writable]` The permission user account that needs to be created,
    ///      belongs to the first operator in the group
    ///   3. `[]` The initial operator
    ///   4. `[]` The system program
    SetupPermissionGroup {
        /// Unique identifier the the account group
        id: GroupId,
    },
    /// Add a new user account to the permission group.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the account creation
    ///   1. `[]` The permission group account
    ///   2. `[]` The EXISTING permission user account
    ///   3. `[signer]` The owner of the EXISTING permission user account
    ///   4. `[]` The owner of the NEW permission user account
    ///   5. `[writable]` The NEW permission user account for the new user
    ///   6. `[]` The system program
    AddAccountToPermissionGroup,
    /// Transfer a permission from one user to another.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the account creation
    ///   1. `[]` The permission group account
    ///   2. `[writable]` The EXISTING permission user account (will be deleted)
    ///   3. `[signer]` The owner of the EXISTING permission user account
    ///   6. `[]` The system program
    RenouncePermission,
}

impl GroupId {
    /// Create a new `GroupId` from a slice
    pub fn new(group_id: impl AsRef<[u8]>) -> Self {
        let group_id = hash(group_id.as_ref());
        Self(group_id.to_bytes())
    }

    /// Convert `GroupId` to bytes
    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }
}

/// Create `SetupPermissionGroup` instruction
pub fn build_setup_permission_group_instruction(
    funder: &Pubkey,
    operator_group_pda: &Pubkey,
    operator_pda: &Pubkey,
    operator: &Pubkey,
    group_id: GroupId,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&PermissionGroupInstruction::SetupPermissionGroup { id: group_id })?;

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

/// Create `AddAccountToPermissionGroup` instruction
pub fn build_add_account_to_group_instruction(
    funder: &Pubkey,
    existing_permission_group_pda: &Pubkey,
    existing_permission_account_pda: &Pubkey,
    existing_permission_account_pda_owner: &Pubkey,
    new_permission_user_account_pda_owner: &Pubkey,
    new_permission_user_account_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&PermissionGroupInstruction::AddAccountToPermissionGroup)?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new_readonly(*existing_permission_group_pda, false),
        AccountMeta::new_readonly(*existing_permission_account_pda, false),
        AccountMeta::new_readonly(*existing_permission_account_pda_owner, true),
        AccountMeta::new_readonly(*new_permission_user_account_pda_owner, false),
        AccountMeta::new(*new_permission_user_account_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}

/// Create `RenouncePermission` instruction
pub fn build_renounce_permission_instruction(
    existing_permission_group_pda: &Pubkey,
    existing_permission_account_pda: &Pubkey,
    existing_permission_account_pda_owner: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&PermissionGroupInstruction::RenouncePermission)?;

    let accounts = vec![
        AccountMeta::new_readonly(*existing_permission_group_pda, false),
        AccountMeta::new(*existing_permission_account_pda, false),
        AccountMeta::new(*existing_permission_account_pda_owner, true),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}
