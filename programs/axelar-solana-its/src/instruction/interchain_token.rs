//! Instructions for the Interchain Token

use borsh::to_vec;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::system_program;
use spl_associated_token_account::get_associated_token_address_with_program_id;

use super::InterchainTokenServiceInstruction;

/// Creates an [`InterchainTokenServiceInstruction::MintInterchainToken`] instruction.
///
/// # Errors
/// If serialization fails.
pub fn mint(
    payer: Pubkey,
    token_id: [u8; 32],
    mint: Pubkey,
    to: Pubkey,
    minter: Pubkey,
    token_program: Pubkey,
    amount: u64,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &minter);
    let data = to_vec(&InterchainTokenServiceInstruction::MintInterchainToken { amount })?;
    let ata = get_associated_token_address_with_program_id(&to, &mint, &token_program);

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new(mint, false),
            AccountMeta::new(to, false),
            AccountMeta::new(ata, false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new_readonly(token_manager_pda, false),
            AccountMeta::new_readonly(minter, true),
            AccountMeta::new_readonly(minter_roles_pda, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::TransferInterchainTokenMintership`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn transfer_mintership(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (destination_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &to);
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &payer);

    let accounts = vec![
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new(payer_roles_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(destination_roles_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::TransferInterchainTokenMintership)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ProposeInterchainTokenMintership`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn propose_mintership(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &payer);
    let (destination_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &to);
    let (proposal_pda, _) =
        role_management::find_roles_proposal_pda(&crate::id(), &token_manager_pda, &payer, &to);

    let accounts = vec![
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(destination_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::ProposeInterchainTokenMintership)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`Instruction::MinterInstruction`]
/// instruction with the [`minter::Instruction::AcceptMintership`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn accept_mintership(
    payer: Pubkey,
    token_id: [u8; 32],
    from: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &payer);
    let (origin_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &from);
    let (proposal_pda, _) =
        role_management::find_roles_proposal_pda(&crate::id(), &token_manager_pda, &from, &payer);

    let accounts = vec![
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new(payer_roles_pda, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(from, false),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::AcceptInterchainTokenMintership)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
