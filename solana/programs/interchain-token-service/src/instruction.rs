//! Instruction types

use account_group::get_permission_account;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::ethers_core::abi::AbiEncode;
use interchain_token_transfer_gmp::{DeployTokenManager, GMPPayload};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;

use crate::{id, TokenManagerType};

/// Instructions supported by the InterchainTokenService program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Initialize the InterchainTokenService program
    Initialize {},
    /// Execute a GMP payload
    Execute {
        /// GMP payload
        payload: Vec<u8>,
    },
    /// Instruction GiveToken.
    /// This function gives token to a specified address.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [signer] The address of payer.
    /// 1. [writable] The address of the mint token account.
    /// 2. [writable] The address of the token manager.
    /// 3. [] The address of the wallet, known as well as delegate authority.
    /// 4. [writable] The address of the associated token account.
    /// 5. [] The address of the interchain token service root PDA.
    /// 6. [] The address of the gateway root PDA.
    /// 7. [] The address of the gas service root PDA.
    /// 8. [] The address of the SPL token program.
    /// 9. [writable] The address of the SPL associated token account program,
    ///    calculated from the interchain token service root PDA, wallet address
    ///    and mint address.
    GiveToken {
        /// The token manager type.
        token_manager_type: TokenManagerType,
        /// The amount of tokens to give.
        amount: u64,
    },
    /// Instruction TakeToken.
    /// This function takes token from a specified address.
    ///
    /// Accounts expected by this instruction:
    //
    /// 0. [signer] The address of payer.
    /// 1. [writable] The address of the mint token account.
    /// 2. [writable] The address of the token manager.
    /// 3. [] The address of the wallet, known as well as delegate authority.
    /// 4. [writable] The address of the associated token account.
    /// 5. [] The address of the interchain token service root PDA.
    /// 6. [] The address of the gateway root PDA.
    /// 7. [] The address of the gas service root PDA.
    /// 8. [] The address of the SPL token program.
    /// 9. [writable] The address of the SPL associated token account program,
    ///    calculated from the interchain token service root PDA, wallet address
    ///    and mint address.
    TakeToken {
        /// The token manager type.
        token_manager_type: TokenManagerType,
        /// The amount of tokens to give.
        amount: u64,
    },
}

/// Builds a `Setup` instruction for the `TokenManager` program.
///
/// # Returns
///
/// * `Instruction` - The `Setup` instruction for the `TokenManager` program.
///
/// # Errors
///
/// Will return `ProgramError` if the instruction data cannot be serialized.
#[allow(clippy::too_many_arguments)]
pub fn build_initialize_instruction(
    funder: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::Initialize {})?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create a generic `Execute` instruction
pub fn build_execute_instruction(
    funder: &Pubkey,
    incoming_accounts: &[AccountMeta],
    payload: impl AbiEncode,
) -> Result<Instruction, ProgramError> {
    let payload = payload.encode();
    let init_data = InterchainTokenServiceInstruction::Execute { payload };
    let data = to_vec(&init_data)?;

    let mut accounts = vec![AccountMeta::new(*funder, true)];
    accounts.extend_from_slice(incoming_accounts);

    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}

/// Create `Execute::DeployTokenManager` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_deploy_token_manager_instruction(
    funder: &Pubkey,
    token_manager_root_pda: &Pubkey,
    operators_permission_group_pda: &Pubkey,
    operators_permission_pda_owner: &Pubkey,
    flow_limiters_permission_group_pda: &Pubkey,
    flow_limiters_permission_pda_owner: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    token_mint: &Pubkey,
    payload: DeployTokenManager,
) -> Result<Instruction, ProgramError> {
    let token_manager_ata = get_associated_token_address(token_manager_root_pda, token_mint);
    let operators_permission_pda = get_permission_account(
        operators_permission_group_pda,
        operators_permission_pda_owner,
    );
    let flow_limiters_permission_pda = get_permission_account(
        flow_limiters_permission_group_pda,
        flow_limiters_permission_pda_owner,
    );

    build_execute_instruction(
        funder,
        &[
            AccountMeta::new(*token_manager_root_pda, false),
            AccountMeta::new(*operators_permission_group_pda, false),
            AccountMeta::new(operators_permission_pda, false),
            AccountMeta::new_readonly(*operators_permission_pda_owner, false),
            AccountMeta::new(*flow_limiters_permission_group_pda, false),
            AccountMeta::new(flow_limiters_permission_pda, false),
            AccountMeta::new_readonly(*flow_limiters_permission_pda_owner, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(*interchain_token_service_root_pda, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(account_group::id(), false),
            AccountMeta::new_readonly(token_manager::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        GMPPayload::DeployTokenManager(payload),
    )
}

/// Create `GiveToken:MintBurn` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_give_token_mint_burn_instruction(
    amount: u64,
    payer: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    delegate_authority: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::GiveToken {
        token_manager_type: TokenManagerType::MintBurn,
        amount,
    })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*mint_account_pda, false),
        AccountMeta::new_readonly(*delegate_authority, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create `GiveToken:LockUnlock` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_give_token_lock_unlock_instruction(
    amount: u64,
    payer: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    token_manager_ata_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    destination: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::GiveToken {
        token_manager_type: TokenManagerType::LockUnlock,
        amount,
    })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*token_manager_ata_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new_readonly(*mint_account_pda, false),
        AccountMeta::new_readonly(*destination, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create `TakeToken:MintBurn` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_take_token_mint_burn_instruction(
    amount: u64,
    payer: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    delegate_authority: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::TakeToken {
        token_manager_type: TokenManagerType::MintBurn,
        amount,
    })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*mint_account_pda, false),
        AccountMeta::new_readonly(*delegate_authority, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create `TakeToken:LockUnlock` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_take_token_lock_unlock_instruction(
    amount: u64,
    payer: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    token_manager_ata_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    destination: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::TakeToken {
        token_manager_type: TokenManagerType::LockUnlock,
        amount,
    })?;

    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*token_manager_ata_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new_readonly(*mint_account_pda, false),
        AccountMeta::new_readonly(*destination, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}
