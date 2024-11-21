//! Instructions for the token manager.

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use role_management::instructions::{RoleManagementInstruction, RoleManagementInstructionInputs};
use role_management::state::Roles;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::InterchainTokenServiceInstruction;

/// Instructions operating on [`TokenManager`] instances.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Instruction {
    /// Adds a flow limiter to a [`TokenManager`].
    AddFlowLimiter(RoleManagementInstructionInputs),

    /// Removes a flow limiter from a [`TokenManager`].
    RemoveFlowLimiter(RoleManagementInstructionInputs),

    /// Sets the flow limit for an interchain token.
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: u64,
    },
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
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::id());
    let (interchain_token_pdas, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pdas);

    let instruction = role_management::instructions::add_roles::<Instruction>(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
    )?;

    Ok(instruction)
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
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::id());
    let (interchain_token_pdas, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pdas);

    let instruction = role_management::instructions::remove_roles::<Instruction>(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
    )?;

    Ok(instruction)
}
