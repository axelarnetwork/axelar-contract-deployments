//! Instructions for the Interchain Token

use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{minter, InterchainTokenServiceInstruction};

/// Instructions operating on [`TokenManager`] instances.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(CheckBytes))]
pub enum Instruction {
    /// A proxy instruction to mint tokens whose mint authority is a
    /// `TokenManager`. Only users with the `minter` role on the mint account
    /// can mint tokens.
    ///
    /// 0. [writable] The mint account
    /// 1. [writable] The account to mint tokens to
    /// 2. [] The interchain token PDA associated with the mint
    /// 3. [] The token manager PDA
    /// 4. [signer] The minter account
    /// 5. [] The token program id
    Mint {
        /// The amount of tokens to mint.
        amount: u64,
    },

    /// `TokenManager` instructions to manage Operator role.
    ///
    /// 0. [] Interchain Token PDA.
    /// 1..N [`minter::MinterInstruction`] accounts, where the resource is
    /// the Interchain Token PDA.
    MinterInstruction(super::minter::Instruction),
}

/// Creates an [`InterchainTokenServiceInstruction::InterchainTokenInstruction`]
/// instruction with the [`Instruction::Mint`] variant.
///
/// # Errors
/// If serialization fails.
pub fn mint(
    token_id: [u8; 32],
    mint: Pubkey,
    to: Pubkey,
    minter: Pubkey,
    token_program: Pubkey,
    amount: u64,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &minter);
    let instruction =
        InterchainTokenServiceInstruction::InterchainTokenInstruction(Instruction::Mint { amount });
    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(solana_program::instruction::Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(mint, false),
            AccountMeta::new(to, false),
            AccountMeta::new_readonly(interchain_token_pda, false),
            AccountMeta::new_readonly(token_manager_pda, false),
            AccountMeta::new_readonly(minter, true),
            AccountMeta::new_readonly(minter_roles_pda, false),
            AccountMeta::new_readonly(token_program, false),
        ],
        data,
    })
}

/// Creates an [`Instruction::MinterInstruction`]
/// instruction with the [`minter::Instruction::TransferMintership`]
/// variant.
///
/// # Errors
///
/// If serialization fails.
pub fn transfer_mintership(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, minter_instruction) =
        minter::transfer_mintership(payer, token_manager_pda, to, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::InterchainTokenInstruction(
        Instruction::MinterInstruction(minter_instruction),
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

/// Creates an [`Instruction::MinterInstruction`]
/// instruction with the [`minter::Instruction::ProposeMintership`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn propose_mintership(
    payer: Pubkey,
    token_id: [u8; 32],
    to: Pubkey,
) -> Result<solana_program::instruction::Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, minter_instruction) =
        minter::propose_mintership(payer, token_manager_pda, to, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::InterchainTokenInstruction(
        Instruction::MinterInstruction(minter_instruction),
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
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);
    let accounts = vec![AccountMeta::new_readonly(interchain_token_pda, false)];
    let (accounts, minter_instruction) =
        minter::accept_mintership(payer, token_manager_pda, from, Some(accounts))?;
    let instruction = InterchainTokenServiceInstruction::InterchainTokenInstruction(
        Instruction::MinterInstruction(minter_instruction),
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
