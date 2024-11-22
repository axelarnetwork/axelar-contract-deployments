//! Instructions for the token manager.

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use role_management::instructions::{RoleManagementInstruction, RoleManagementInstructionInputs};
use role_management::state::Roles;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{operator, InterchainTokenServiceInstruction};

/// Instructions operating on [`TokenManager`] instances.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Instruction {
    /// Adds a flow limiter to a [`TokenManager`].
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA account for the payer roles on the [`TokenManager`].
    /// 3. [] PDA account for the [`TokenManager`].
    /// 4. [] Account to add the Flow Limiter role to.
    /// 5. [writable] PDA account with the roles on the [`TokenManager`], for
    ///    the accounts the roles are being added to.
    AddFlowLimiter(RoleManagementInstructionInputs),

    /// Removes a flow limiter from a [`TokenManager`].
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA account for the payer roles on the [`TokenManager`].
    /// 3. [] PDA account for the [`TokenManager`].
    /// 4. [] Account to remove the Flow Limiter role from.
    /// 5. [writable] PDA account with the roles on the [`TokenManager`], for
    ///    the accounts the roles are being added to.
    RemoveFlowLimiter(RoleManagementInstructionInputs),

    /// Sets the flow limit for an interchain token.
    ///
    /// 0. [signer] Payer account.
    /// 1. [] ITS root PDA account.
    /// 2. [writable] The [`TokenManager`] PDA account.
    /// 3. [] The PDA account with the user roles on the [`TokenManager`].
    /// 4. [] The PDA account with the user roles on ITS.
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: u64,
    },

    /// `TokenManager` instructions to manage Operator role.
    ///
    /// 0. [] Interchain Token PDA.
    /// 1..N [`operator::OperatorInstruction`] accounts, where the resource is
    /// the [`TokenManager`] PDA.
    OperatorInstruction(super::operator::Instruction),
}

impl TryFrom<RoleManagementInstruction> for Instruction {
    type Error = ProgramError;
    fn try_from(value: RoleManagementInstruction) -> Result<Self, Self::Error> {
        match value {
            RoleManagementInstruction::AddRoles(inputs) => Ok(Self::AddFlowLimiter(inputs)),
            RoleManagementInstruction::RemoveRoles(inputs) => Ok(Self::RemoveFlowLimiter(inputs)),
            RoleManagementInstruction::TransferRoles(_)
            | RoleManagementInstruction::ProposeRoles(_)
            | RoleManagementInstruction::AcceptRoles(_) => {
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}

/// Creates an [`TokenManagerInstructions::SetFlowLimit`] wrapped in an
/// [`InterchainTokenServiceInstruction::TokenManagerInstruction`].
///
/// # Errors
///
/// If serialization fails.
pub fn set_flow_limit(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limit: u64,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::get_gateway_root_config_pda().0);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &payer);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);

    let instruction =
        InterchainTokenServiceInstruction::TokenManagerInstruction(Instruction::SetFlowLimit {
            flow_limit,
        });

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(token_manager_user_roles_pda, false),
        AccountMeta::new_readonly(its_user_roles_pda, false),
    ];

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`TokenManagerInstructions::AddFlowLimiter`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn add_flow_limiter(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limiter: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::get_gateway_root_config_pda().0);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);

    let (accounts, role_management_instruction) = role_management::instructions::add_roles(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
        None,
    );

    let instruction = InterchainTokenServiceInstruction::TokenManagerInstruction(
        role_management_instruction.try_into()?,
    );
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`TokenManagerInstructions::RemoveFlowLimiter`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn remove_flow_limiter(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limiter: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::get_gateway_root_config_pda().0);
    let (interchain_token_pdas, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pdas);

    let (accounts, role_management_instruction) = role_management::instructions::remove_roles(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
        None,
    );
    let instruction = InterchainTokenServiceInstruction::TokenManagerInstruction(
        role_management_instruction.try_into()?,
    );
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`Instruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::TransferOperatorship`]
/// variant.
///
/// # Errors
///
/// If serialization fails.
pub fn transfer_operatorship(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, operator_instruction) =
        operator::transfer_operatorship(payer, token_manager_pda, to, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::TokenManagerInstruction(
        Instruction::OperatorInstruction(operator_instruction),
    );
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`Instruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::ProposeOperatorship`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn propose_operatorship(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, operator_instruction) =
        operator::propose_operatorship(payer, token_manager_pda, to, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::TokenManagerInstruction(
        Instruction::OperatorInstruction(operator_instruction),
    );
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`Instruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::AcceptOperatorship`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn accept_operatorship(
    payer: Pubkey,
    token_id: [u8; 32],
    from: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, operator_instruction) =
        operator::accept_operatorship(payer, token_manager_pda, from, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::TokenManagerInstruction(
        Instruction::OperatorInstruction(operator_instruction),
    );
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
