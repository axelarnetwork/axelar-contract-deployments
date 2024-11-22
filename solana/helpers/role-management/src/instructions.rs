//! Instructions for role management.
use std::error::Error;

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use crate::state::archive::ArchivableRoles;
use crate::state::Roles;

/// Inputs for role management related instructions.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct RoleManagementInstructionInputs {
    /// The roles to add or transfer.
    #[with(ArchivableRoles)]
    pub roles: Roles,

    /// The bump for the destination roles PDA.
    pub destination_roles_pda_bump: u8,

    /// The bump for the proposal PDA used by the instruction, if any.
    pub proposal_pda_bump: Option<u8>,
}

/// Role management instructions.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum RoleManagementInstruction {
    /// Adds roles to a user.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA account for the payer roles on the resource.
    /// 3. [] PDA account for the resource.
    /// 4. [] Account to add roles to.
    /// 5. [writable] PDA account with the roles on the resource, for the
    ///    accounts the roles are being added to.
    AddRoles(RoleManagementInstructionInputs),

    /// Removes roles from a user.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA account for the payer roles on the resource.
    /// 3. [] PDA account for the resource.
    /// 4. [] Account to remove roles from.
    /// 5. [writable] PDA account with the roles on the resource, for the
    ///    accounts the roles are being removed from.
    RemoveRoles(RoleManagementInstructionInputs),

    /// Transfers roles from one user to another.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer roles to.
    /// 5. [writable] PDA with the roles on the resource, for the accounts the
    ///    roles are being transferred to.
    /// 6. [] Account which the roles are being transferred from.
    /// 7. [writable] PDA with the roles on the resource, for the account the
    ///    roles are being transferred from.
    TransferRoles(RoleManagementInstructionInputs),

    /// Proposes roles to a user. Upon acceptance the roles are transferred.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer roles to.
    /// 5. [] PDA with the roles on the resource, for the accounts the roles are
    ///    being transferred to.
    /// 6. [] Account which the roles are being transferred from.
    /// 7. [] PDA with the roles on the resource, for the account the roles are
    ///    being transferred from.
    /// 8. [writable] The PDA account containing the proposal.
    ProposeRoles(RoleManagementInstructionInputs),

    /// Accepts proposed roles.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer roles to.
    /// 5. [writable] PDA with the roles on the resource, for the accounts the
    ///    roles are being transferred to.
    /// 6. [] Account which the roles are being transferred from.
    /// 7. [writable] PDA with the roles on the resource, for the account the
    ///    roles are being transferred from.
    /// 8. [writable] The PDA account containing the proposal.
    AcceptRoles(RoleManagementInstructionInputs),
}

impl RoleManagementInstruction {
    /// Serializes the instruction into a byte array.
    ///
    /// # Errors
    ///
    /// If serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let bytes = rkyv::to_bytes::<_, 0>(self).map_err(Box::new)?;

        Ok(bytes.to_vec())
    }

    /// Deserializes the instruction from a byte array.
    ///
    /// # Errors
    ///
    /// If deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // SAFETY:
        // - The byte slice represents an archived object
        // - The root of the object is stored at the end of the slice
        let bytes = unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }.map_err(Box::new)?;

        Ok(bytes)
    }
}

/// Creates an instruction to add roles to a user.
#[must_use]
pub fn add_roles(
    program_id: Pubkey,
    payer: Pubkey,
    on: Pubkey,
    to: Pubkey,
    roles: Roles,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> (Vec<AccountMeta>, RoleManagementInstruction) {
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (payer_roles_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: None,
    };

    let instruction = RoleManagementInstruction::AddRoles(inputs);

    let mut accounts = accounts_to_prepend.unwrap_or_default();

    accounts.append(&mut vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(destination_roles_pda, false),
    ]);

    (accounts, instruction)
}

/// Creates an instruction to remove roles from a user.
#[must_use]
pub fn remove_roles(
    program_id: Pubkey,
    payer: Pubkey,
    on: Pubkey,
    from: Pubkey,
    roles: Roles,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> (Vec<AccountMeta>, RoleManagementInstruction) {
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &from);
    let (payer_roles_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: None,
    };

    let instruction = RoleManagementInstruction::RemoveRoles(inputs);

    let mut accounts = accounts_to_prepend.unwrap_or_default();
    accounts.append(&mut vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(from, false),
        AccountMeta::new(destination_roles_pda, false),
    ]);

    (accounts, instruction)
}

/// Creates an instruction to transfer roles between users.
#[must_use]
pub fn transfer_roles(
    program_id: Pubkey,
    payer: Pubkey,
    on: Pubkey,
    from: Pubkey,
    to: Pubkey,
    roles: Roles,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> (Vec<AccountMeta>, RoleManagementInstruction) {
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (payer_roles_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &from);
    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: None,
    };

    let instruction = RoleManagementInstruction::TransferRoles(inputs);

    let mut accounts = accounts_to_prepend.unwrap_or_default();
    accounts.append(&mut vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new_readonly(from, false),
        AccountMeta::new(role_holder_pda, false),
    ]);

    (accounts, instruction)
}

/// Creates an instruction to transfer roles between users.
#[must_use]
pub fn propose_roles(
    program_id: Pubkey,
    payer: Pubkey,
    on: Pubkey,
    from: Pubkey,
    to: Pubkey,
    roles: Roles,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> (Vec<AccountMeta>, RoleManagementInstruction) {
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (payer_roles_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &from);
    let (proposal_pda, proposal_pda_bump) =
        crate::find_roles_proposal_pda(&program_id, &on, &payer, &to);

    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: Some(proposal_pda_bump),
    };

    let instruction = RoleManagementInstruction::ProposeRoles(inputs);

    let mut accounts = accounts_to_prepend.unwrap_or_default();
    accounts.append(&mut vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new_readonly(destination_roles_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(role_holder_pda, false),
        AccountMeta::new(proposal_pda, false),
    ]);

    (accounts, instruction)
}

/// Creates an instruction to transfer roles between users.
#[must_use]
pub fn accept_roles(
    program_id: Pubkey,
    payer: Pubkey,
    on: Pubkey,
    from: Pubkey,
    roles: Roles,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> (Vec<AccountMeta>, RoleManagementInstruction) {
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &payer);
    let (payer_roles_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &from);
    let (proposal_pda, proposal_pda_bump) =
        crate::find_roles_proposal_pda(&program_id, &on, &from, &payer);

    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: Some(proposal_pda_bump),
    };

    let instruction = RoleManagementInstruction::AcceptRoles(inputs);

    let mut accounts = accounts_to_prepend.unwrap_or_default();
    accounts.append(&mut vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new(payer, true),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new(from, false),
        AccountMeta::new(role_holder_pda, false),
        AccountMeta::new(proposal_pda, false),
    ]);

    (accounts, instruction)
}
