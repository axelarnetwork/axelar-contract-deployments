//! Instructions for the token manager.

use borsh::to_vec;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;

use super::{operator, InterchainTokenServiceInstruction};
use crate::Roles;

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
    let (its_root_pda, _) =
        crate::find_its_root_pda(&axelar_solana_gateway::get_gateway_root_config_pda().0);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &payer);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);

    let data = to_vec(&InterchainTokenServiceInstruction::TokenManagerSetFlowLimit { flow_limit })?;

    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(token_manager_user_roles_pda, false),
        AccountMeta::new_readonly(its_user_roles_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
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
    let (its_root_pda, _) =
        crate::find_its_root_pda(&axelar_solana_gateway::get_gateway_root_config_pda().0);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (accounts, inputs) = role_management::instructions::add_roles(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
        None,
    );

    let data = to_vec(&InterchainTokenServiceInstruction::TokenManagerAddFlowLimiter { inputs })?;

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
    let (its_root_pda, _) =
        crate::find_its_root_pda(&axelar_solana_gateway::get_gateway_root_config_pda().0);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (accounts, inputs) = role_management::instructions::remove_roles(
        crate::id(),
        payer,
        token_manager_pda,
        flow_limiter,
        Roles::FLOW_LIMITER,
        None,
    );
    let data =
        to_vec(&InterchainTokenServiceInstruction::TokenManagerRemoveFlowLimiter { inputs })?;

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
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let accounts = vec![AccountMeta::new_readonly(its_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::transfer_operatorship(payer, token_manager_pda, to, Some(accounts))?;

    let inputs = match operator_instruction {
        operator::Instruction::TransferOperatorship(val) => val,
        operator::Instruction::ProposeOperatorship(_)
        | operator::Instruction::AcceptOperatorship(_) => {
            return Err(ProgramError::InvalidInstructionData)
        }
    };
    let data =
        to_vec(&InterchainTokenServiceInstruction::TokenManagerTransferOperatorship { inputs })?;

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
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let accounts = vec![AccountMeta::new_readonly(its_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::propose_operatorship(payer, token_manager_pda, to, Some(accounts))?;

    let inputs = match operator_instruction {
        operator::Instruction::ProposeOperatorship(val) => val,
        operator::Instruction::TransferOperatorship(_)
        | operator::Instruction::AcceptOperatorship(_) => {
            return Err(ProgramError::InvalidInstructionData)
        }
    };
    let data =
        to_vec(&InterchainTokenServiceInstruction::TokenManagerProposeOperatorship { inputs })?;

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
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let accounts = vec![AccountMeta::new_readonly(its_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::accept_operatorship(payer, token_manager_pda, from, Some(accounts))?;

    let inputs = match operator_instruction {
        operator::Instruction::AcceptOperatorship(val) => val,
        operator::Instruction::TransferOperatorship(_)
        | operator::Instruction::ProposeOperatorship(_) => {
            return Err(ProgramError::InvalidInstructionData)
        }
    };
    let data =
        to_vec(&InterchainTokenServiceInstruction::TokenManagerAcceptOperatorship { inputs })?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`Instruction::HandoverMintAuthority`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn handover_mint_authority(
    payer: Pubkey,
    token_id: [u8; 32],
    mint: Pubkey,
    token_program: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &payer);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(minter_roles_pda, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data =
        to_vec(&InterchainTokenServiceInstruction::TokenManagerHandOverMintAuthority { token_id })?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
