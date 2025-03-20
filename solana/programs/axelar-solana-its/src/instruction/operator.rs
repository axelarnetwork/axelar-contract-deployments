//! Instructions to manage the operator role.

use borsh::{BorshDeserialize, BorshSerialize};
use role_management::instructions::{RoleManagementInstruction, RoleManagementInstructionInputs};
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::Roles;

/// Instructions to manage the operator role.
#[derive(Debug, PartialEq, Eq, Clone, BorshSerialize, BorshDeserialize)]
pub enum Instruction {
    /// Transfers operatorship to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    TransferOperatorship(RoleManagementInstructionInputs<Roles>),

    /// Proposes operatorship transfer to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    ProposeOperatorship(RoleManagementInstructionInputs<Roles>),

    /// Accepts operatorship transfer from another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    AcceptOperatorship(RoleManagementInstructionInputs<Roles>),
}

impl TryFrom<RoleManagementInstruction<Roles>> for Instruction {
    type Error = ProgramError;
    fn try_from(value: RoleManagementInstruction<Roles>) -> Result<Self, Self::Error> {
        match value {
            RoleManagementInstruction::TransferRoles(inputs) => {
                Ok(Self::TransferOperatorship(inputs))
            }
            RoleManagementInstruction::ProposeRoles(inputs) => {
                Ok(Self::ProposeOperatorship(inputs))
            }
            RoleManagementInstruction::AcceptRoles(inputs) => Ok(Self::AcceptOperatorship(inputs)),
            RoleManagementInstruction::AddRoles(_) | RoleManagementInstruction::RemoveRoles(_) => {
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}

/// Creates a [`Instruction::Transfer`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub(crate) fn transfer_operatorship(
    payer: Pubkey,
    on: Pubkey,
    to: Pubkey,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> Result<(Vec<AccountMeta>, Instruction), ProgramError> {
    let (accounts, instruction) = role_management::instructions::transfer_roles(
        crate::id(),
        payer,
        on,
        payer,
        to,
        Roles::OPERATOR,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}

/// Creates a [`Instruction::Propose`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub(crate) fn propose_operatorship(
    payer: Pubkey,
    on: Pubkey,
    to: Pubkey,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> Result<(Vec<AccountMeta>, Instruction), ProgramError> {
    let (accounts, instruction) = role_management::instructions::propose_roles(
        crate::id(),
        payer,
        on,
        payer,
        to,
        Roles::OPERATOR,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}

/// Creates a [`Instruction::Accept`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub(crate) fn accept_operatorship(
    payer: Pubkey,
    on: Pubkey,
    from: Pubkey,
    accounts_to_prepend: Option<Vec<AccountMeta>>,
) -> Result<(Vec<AccountMeta>, Instruction), ProgramError> {
    let (accounts, instruction) = role_management::instructions::accept_roles(
        crate::id(),
        payer,
        on,
        from,
        Roles::OPERATOR,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}
