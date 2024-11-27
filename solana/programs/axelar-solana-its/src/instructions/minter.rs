//! Instructions to manage the minter role.

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use role_management::instructions::{RoleManagementInstruction, RoleManagementInstructionInputs};
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::Roles;

/// Instructions to manage the operator role.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(CheckBytes))]
pub enum Instruction {
    /// Transfers mintership to another account.
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
    TransferMintership(RoleManagementInstructionInputs<Roles>),

    /// Proposes mintership transfer to another account.
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
    ProposeMintership(RoleManagementInstructionInputs<Roles>),

    /// Accepts mintership transfer from another account.
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
    AcceptMintership(RoleManagementInstructionInputs<Roles>),
}

impl TryFrom<RoleManagementInstruction<Roles>> for Instruction {
    type Error = ProgramError;
    fn try_from(value: RoleManagementInstruction<Roles>) -> Result<Self, Self::Error> {
        match value {
            RoleManagementInstruction::TransferRoles(inputs) => {
                Ok(Self::TransferMintership(inputs))
            }
            RoleManagementInstruction::ProposeRoles(inputs) => Ok(Self::ProposeMintership(inputs)),
            RoleManagementInstruction::AcceptRoles(inputs) => Ok(Self::AcceptMintership(inputs)),
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
pub(crate) fn transfer_mintership(
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
        Roles::MINTER,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}

/// Creates a [`Instruction::Propose`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub(crate) fn propose_mintership(
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
        Roles::MINTER,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}

/// Creates a [`Instruction::Accept`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub(crate) fn accept_mintership(
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
        Roles::MINTER,
        accounts_to_prepend,
    );

    Ok((accounts, instruction.try_into()?))
}
