//! Instruction types

use account_group::get_permission_account;
use axelar_executable::axelar_message_primitives::AxelarCallableInstruction;
use axelar_message_primitives::DataPayload;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::ethers_core::abi::AbiEncode;
use interchain_token_transfer_gmp::DeployTokenManager;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use token_manager::TokenManagerType;

use crate::MetadataVersion;

/// Instructions supported by the InterchainTokenService program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Initialize the InterchainTokenService program
    Initialize {},
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
    /// Instruction DeployRemoteTokenManager.
    /// Used to deploy remote custom TokenManagers.
    ///
    /// Accounts expected by this instruction:
    //
    /// 0. [signer] The address of payer/ sender.
    DeployRemoteTokenManager {
        /// The salt to be used during deployment.
        salt: [u8; 32],
        /// The name of the chain to deploy the TokenManager and standardized
        /// token to.
        destination_chain: Vec<u8>,
        /// The token manager to deploy.
        token_manager_type: TokenManagerType,
        /// The params that will be used to initialize the TokenManager.
        params: Vec<u8>,
        /// The amount of native tokens to be used to pay for gas for the remote
        /// deployment.
        gas_value: u64,
    },
    /// Instruction DeployRemoteInterchainToken.
    /// Used to deploy remote interchain tokens.
    ///
    /// Accounts expected by this instruction:
    //
    /// 0. [signer] The address of payer/ sender.
    DeployRemoteInterchainToken {
        /// The salt to be used during deployment.
        salt: [u8; 32],
        /// The name of the destination chain to deploy to.
        destination_chain: Vec<u8>,
        /// The name of the token to be deployed.
        name: String,
        /// The symbol of the token to be deployed.
        symbol: String,
        /// The decimals of the token to be deployed.
        decimals: u8,
        /// The address that will be able to mint and burn the deployed token.
        minter: Vec<u8>,
        /// The amount of native tokens to be used to pay for gas for the remote
        /// deployment.
        gas_value: u64,
    },

    /// Instruction RemoteInterchainTransfer
    /// used for transferring tokens to an account on a different chain
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [signer] The address of payer / sender.
    RemoteInterchainTransfer {
        /// The unique identifier of the token to be transferred.
        token_id: [u8; 32],
        /// The destination chain to send the tokens to.
        destination_chain: Vec<u8>,
        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,
        /// The amount of tokens to be transferred.
        amount: u64,
        /// The optional metadata for the call for additional effects (such as
        /// calling a destination contract).
        data: Vec<u8>,
        /// Metadata version
        metadata_version: MetadataVersion,
        /// The token symbol / extra Solana field
        symbol: Vec<u8>,
        /// The token manager type / extra solana field
        token_manager_type: TokenManagerType,
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
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::Initialize {},
    ))?;

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
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::GiveToken {
            token_manager_type: TokenManagerType::MintBurn,
            amount,
        },
    ))?;

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
    let ix = AxelarCallableInstruction::Native(InterchainTokenServiceInstruction::GiveToken {
        token_manager_type: TokenManagerType::LockUnlock,
        amount,
    });
    let data = to_vec(&ix)?;

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
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::TakeToken {
            token_manager_type: TokenManagerType::MintBurn,
            amount,
        },
    ))?;

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
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::TakeToken {
            token_manager_type: TokenManagerType::LockUnlock,
            amount,
        },
    ))?;

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

/// Create [`InterchainTokenServiceInstruction::DeployRemoteTokenManager`]
/// instruction
#[allow(clippy::too_many_arguments)]
pub fn build_deploy_remote_token_manager_instruction(
    sender: &Pubkey,
    salt: [u8; 32],
    destination_chain: Vec<u8>,
    token_manager_type: TokenManagerType,
    params: Vec<u8>,
    gas_value: u64,
    associated_trusted_address: &Pubkey,
    gateway_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::DeployRemoteTokenManager {
            salt,
            destination_chain,
            token_manager_type,
            params,
            gas_value,
        },
    ))?;

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gas_service::id(), false),
        AccountMeta::new(gas_service::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(*associated_trusted_address, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(gateway::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create `DeployRemoteInterchainToken` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_deploy_remote_interchain_token_instruction(
    sender: &Pubkey,
    salt: [u8; 32],
    destination_chain: Vec<u8>,
    name: String,
    symbol: String,
    decimals: u8,
    minter: Vec<u8>,
    gas_value: u64,
    associated_trusted_address: &Pubkey,
    gateway_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            name,
            symbol,
            decimals,
            minter,
            gas_value,
        },
    ))?;

    let accounts = vec![
        AccountMeta::new(*sender, true),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gas_service::id(), false),
        AccountMeta::new(gas_service::get_gas_service_root_pda().0, false),
        AccountMeta::new_readonly(*associated_trusted_address, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(gateway::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Create `RemoteInterchainTransfer::MintBurn` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_remote_interchain_transfer_mint_burn_instruction(
    sender: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    delegate_authority: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_flow_pda: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    flow_limiter_pda: &Pubkey,
    flow_limiter: &Pubkey,
    permission_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
    token_id: [u8; 32],
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    amount: u64,
    data: Vec<u8>,
    metadata_version: MetadataVersion,
    symbol: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::RemoteInterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            metadata_version,
            symbol,
            token_manager_type: TokenManagerType::MintBurn,
        },
    ))?;

    let accounts = vec![
        AccountMeta::new(*sender, true),
        // Take Token
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*mint_account_pda, false),
        AccountMeta::new_readonly(*delegate_authority, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        // Add Flow
        AccountMeta::new_readonly(*token_manager_pda, false),
        AccountMeta::new(*token_manager_flow_pda, false),
        AccountMeta::new_readonly(*flow_limiter_group_pda, false),
        AccountMeta::new_readonly(*flow_limiter_pda, false),
        AccountMeta::new_readonly(*flow_limiter, true),
        AccountMeta::new_readonly(*permission_group_pda, false),
        AccountMeta::new_readonly(*service_program_pda, false),
        // Ours
        AccountMeta::new_readonly(crate::id(), false),
        AccountMeta::new_readonly(token_manager::id(), false),
        AccountMeta::new_readonly(gateway::id(), false),
        // System
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

/// Create `RemoteInterchainTransfer::LockUnlock` instruction
#[allow(clippy::too_many_arguments)]
pub fn build_remote_interchain_transfer_lock_unlock_instruction(
    sender: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    token_manager_ata_pda: &Pubkey,
    owner_of_its_ata_for_user_tokens_pda: &Pubkey,
    its_ata_for_user_tokens_pda: &Pubkey,
    mint_account_pda: &Pubkey,
    destination: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    token_manager_pda: &Pubkey,
    token_manager_flow_pda: &Pubkey,
    flow_limiter_group_pda: &Pubkey,
    flow_limiter_pda: &Pubkey,
    flow_limiter: &Pubkey,
    permission_group_pda: &Pubkey,
    service_program_pda: &Pubkey,
    token_id: [u8; 32],
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    amount: u64,
    data: Vec<u8>,
    metadata_version: MetadataVersion,
    symbol: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&AxelarCallableInstruction::Native(
        InterchainTokenServiceInstruction::RemoteInterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            metadata_version,
            symbol,
            token_manager_type: TokenManagerType::LockUnlock,
        },
    ))?;

    let accounts = vec![
        AccountMeta::new(*sender, true),
        // Take Token
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new(*token_manager_ata_pda, false),
        AccountMeta::new(*owner_of_its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*its_ata_for_user_tokens_pda, false),
        AccountMeta::new(*mint_account_pda, false),
        AccountMeta::new_readonly(*destination, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        // Add Flow
        AccountMeta::new_readonly(*token_manager_pda, false),
        AccountMeta::new(*token_manager_flow_pda, false),
        AccountMeta::new_readonly(*flow_limiter_group_pda, false),
        AccountMeta::new_readonly(*flow_limiter_pda, false),
        AccountMeta::new_readonly(*flow_limiter, true),
        AccountMeta::new_readonly(*permission_group_pda, false),
        AccountMeta::new_readonly(*service_program_pda, false),
        // Ours
        AccountMeta::new_readonly(crate::id(), false),
        AccountMeta::new_readonly(token_manager::id(), false),
        AccountMeta::new_readonly(gateway::id(), false),
        // System
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

/// Utilites for creating messages that are received from external chains
///
/// Useful in tests for creating messages that are received from external chains
pub mod from_external_chains {
    use std::borrow::Cow;

    use axelar_message_primitives::SolanaAccountRepr;
    use interchain_token_transfer_gmp::{DeployInterchainToken, InterchainTransfer};

    use super::*;

    /// Create a generic `Execute` instruction
    fn build_execute_instruction<'a>(
        payload: impl AbiEncode,
        its_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
        incoming_accounts: &[AccountMeta],
    ) -> DataPayload<'a> {
        let mut accounts = vec![
            // Our program also does generic validation of the ITS root PDA beforethe payload gets
            // procesed and the message is executed. Hence why the next accounts are common to all
            // GMP payloads.
            AccountMeta::new_readonly(*its_root_pda, false),
            AccountMeta::new_readonly(*gas_service_root_pda, false),
        ];
        accounts.extend_from_slice(incoming_accounts);
        let accounts = accounts
            .into_iter()
            .map(|acc| acc.into())
            .collect::<Vec<SolanaAccountRepr>>();

        DataPayload::new_with_cow(Cow::Owned(payload.encode()), accounts)
    }

    /// Create `Execute::DeployTokenManager` instruction
    #[allow(clippy::too_many_arguments)]
    pub fn build_deploy_token_manager_from_gmp_instruction(
        its_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
        funder: &Pubkey,
        token_manager_root_pda: &Pubkey,
        operators_permission_group_pda: &Pubkey,
        operators_permission_pda_owner: &Pubkey,
        flow_limiters_permission_group_pda: &Pubkey,
        flow_limiters_permission_pda_owner: &Pubkey,
        token_mint: &Pubkey,
        payload: DeployTokenManager,
    ) -> DataPayload<'static> {
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
            payload,
            its_root_pda,
            gas_service_root_pda,
            &[
                AccountMeta::new(*funder, true),
                AccountMeta::new(*token_manager_root_pda, false),
                AccountMeta::new(*operators_permission_group_pda, false),
                AccountMeta::new(operators_permission_pda, false),
                AccountMeta::new_readonly(*operators_permission_pda_owner, false),
                AccountMeta::new(*flow_limiters_permission_group_pda, false),
                AccountMeta::new(flow_limiters_permission_pda, false),
                AccountMeta::new_readonly(*flow_limiters_permission_pda_owner, false),
                AccountMeta::new_readonly(*token_mint, false),
                AccountMeta::new(token_manager_ata, false),
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(account_group::id(), false),
                AccountMeta::new_readonly(token_manager::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        )
    }

    /// Create `Execute::DeployInterchainToken` instruction
    #[allow(clippy::too_many_arguments)]
    pub fn build_deploy_interchain_token_from_gmp_instruction(
        its_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
        payload: DeployInterchainToken,
    ) -> DataPayload<'static> {
        build_execute_instruction(
            payload,
            its_root_pda,
            gas_service_root_pda,
            &[
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(account_group::id(), false),
                AccountMeta::new_readonly(token_manager::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        )
    }

    /// Create `Execute::InterchainTransfer` instruction
    #[allow(clippy::too_many_arguments)]
    pub fn build_interchain_transfer_from_gmp_instruction(
        its_root_pda: &Pubkey,
        gas_service_root_pda: &Pubkey,
        payload: InterchainTransfer,
    ) -> DataPayload<'static> {
        build_execute_instruction(
            payload,
            its_root_pda,
            gas_service_root_pda,
            &[
                AccountMeta::new_readonly(solana_program::system_program::id(), false),
                AccountMeta::new_readonly(account_group::id(), false),
                AccountMeta::new_readonly(token_manager::id(), false),
                AccountMeta::new_readonly(spl_associated_token_account::id(), false),
                AccountMeta::new_readonly(spl_token::id(), false),
            ],
        )
    }
}
