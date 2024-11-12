use std::error::Error;

use rkyv::ser::serializers::AllocSerializer;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
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
    /// Adds roles to a user. The signer theyself must have the role as to be
    /// able to add it to other users.
    ///
    /// 0. [] System program id
    /// 1. [signer] The payer account
    /// 2. [] The resource for which the role is being added.
    /// 3. [] The PDA containing the roles for the user who has the rights to
    ///    add the role to other users.
    /// 4. [writeable] The PDA containing the roles for the user to which the
    ///    roles are going to be added.
    /// 5. [signer] The account of the user that has the rights to add the role.
    /// 6. [] The account of the user to which the roles are going to be added.
    AddRoles(RoleManagementInstructionInputs),

    /// Transfers roles from one user to another.
    ///
    /// 0. [] System program id
    /// 1. [signer] The payer account
    /// 2. [] The resource for which the role is being added.
    /// 3. [writeable] The PDA from which the roles being transferred.
    /// 4. [writeable] The PDA containing the roles for the user to which the
    ///    roles are going to be added.
    /// 5. [signer] The account of the user whose roles are being transferred.
    /// 6. [] The account of the user who's going to receive the roles.
    TransferRoles(RoleManagementInstructionInputs),

    /// Proposes roles to a user. Upon acceptance the roles are transferred.
    ///
    /// 0. [] System program id
    /// 1. [signer] The payer account
    /// 2. [] The resource for which the role is being added.
    /// 3. [writeable] The PDA from which the roles being transferred.
    /// 4. [writeable] The PDA containing the roles for the user to which the
    ///    roles are going to be added.
    /// 5. [signer] The account of the user whose roles are being transferred.
    /// 6. [] The account of the user who's going to receive the roles.
    /// 7. [writeable] The PDA containing the proposal.
    ProposeRoles(RoleManagementInstructionInputs),

    /// Accepts proposed roles.
    ///
    /// 0. [] System program id
    /// 1. [signer] The payer account
    /// 2. [] The resource for which the role is being added.
    /// 3. [writeable] The PDA from which the roles being transferred.
    /// 4. [writeable] The PDA containing the roles for the user to which the
    ///    roles are going to be added.
    /// 5. [] The account of the user whose roles are being transferred.
    /// 6. [signer] The account of the user who's going to receive the roles.
    /// 7. [writeable] The PDA containing the proposal.
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
///
/// # Errors
/// If serialization fails.
pub fn add_roles<I>(
    program_id: Pubkey,
    payer: Pubkey,
    to: Pubkey,
    roles: Roles,
    on: Pubkey,
) -> Result<Instruction, ProgramError>
where
    I: From<RoleManagementInstruction> + Archive + Serialize<AllocSerializer<0>>,
{
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: None,
    };

    let instruction: I = RoleManagementInstruction::AddRoles(inputs).into();

    let data = rkyv::to_bytes::<_, 0>(&instruction)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
        .to_vec();

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(role_holder_pda, false),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(to, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates an instruction to transfer roles between users.
///
/// # Errors
/// If serialization fails.
pub fn transfer_roles<I>(
    program_id: Pubkey,
    payer: Pubkey,
    to: Pubkey,
    roles: Roles,
    on: Pubkey,
) -> Result<Instruction, ProgramError>
where
    I: From<RoleManagementInstruction> + Archive + Serialize<AllocSerializer<0>>,
{
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: None,
    };

    let instruction: I = RoleManagementInstruction::TransferRoles(inputs).into();

    let data = rkyv::to_bytes::<_, 0>(&instruction)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
        .to_vec();

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new(role_holder_pda, false),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(to, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates an instruction to transfer roles between users.
///
/// # Errors
/// If serialization fails.
pub fn propose_roles<I>(
    program_id: Pubkey,
    payer: Pubkey,
    to: Pubkey,
    roles: Roles,
    on: Pubkey,
) -> Result<Instruction, ProgramError>
where
    I: From<RoleManagementInstruction> + Archive + Serialize<AllocSerializer<0>>,
{
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &to);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &payer);
    let (proposal_pda, proposal_pda_bump) =
        crate::find_roles_proposal_pda(&program_id, &on, &payer, &to);

    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: Some(proposal_pda_bump),
    };

    let instruction: I = RoleManagementInstruction::ProposeRoles(inputs).into();

    let data = rkyv::to_bytes::<_, 0>(&instruction)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
        .to_vec();

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new_readonly(role_holder_pda, false),
        AccountMeta::new_readonly(destination_roles_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(proposal_pda, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Creates an instruction to transfer roles between users.
///
/// # Errors
/// If serialization fails.
pub fn accept_roles<I>(
    program_id: Pubkey,
    from: Pubkey,
    payer: Pubkey,
    roles: Roles,
    on: Pubkey,
) -> Result<Instruction, ProgramError>
where
    I: From<RoleManagementInstruction> + Archive + Serialize<AllocSerializer<0>>,
{
    let (destination_roles_pda, destination_roles_pda_bump) =
        crate::find_user_roles_pda(&program_id, &on, &payer);
    let (role_holder_pda, _) = crate::find_user_roles_pda(&program_id, &on, &from);
    let (proposal_pda, proposal_pda_bump) =
        crate::find_roles_proposal_pda(&program_id, &on, &from, &payer);

    let inputs = RoleManagementInstructionInputs {
        roles,
        destination_roles_pda_bump,
        proposal_pda_bump: Some(proposal_pda_bump),
    };

    let instruction: I = RoleManagementInstruction::AcceptRoles(inputs).into();

    let data = rkyv::to_bytes::<_, 0>(&instruction)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
        .to_vec();

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(on, false),
        AccountMeta::new(role_holder_pda, false),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new(from, false),
        AccountMeta::new(payer, true),
        AccountMeta::new(proposal_pda, false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}
