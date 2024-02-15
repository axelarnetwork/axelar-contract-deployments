#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod error;
pub mod events;
pub mod instruction;
pub mod processor;
pub mod state;
use account_group::instruction::GroupId;
use interchain_token_transfer_gmp::Bytes32;
pub use solana_program;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Prefix PREFIX_INTERCHAIN_TOKEN_ID
pub const PREFIX_INTERCHAIN_TOKEN_ID: &str = "its-interchain-token-id";

solana_program::declare_id!("4ENH4KjzfcQwyXYr6SJdaF2nhMoGqdZJ2Hk5MoY9mU2G");

/// Derives interchain token service root PDA
pub(crate) fn get_interchain_token_service_root_pda_internal(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&gateway_root_pda.as_ref(), &gas_service_root_pda.as_ref()],
        program_id,
    )
}

/// Derives interchain token service root PDA
pub fn get_interchain_token_service_root_pda(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Pubkey {
    get_interchain_token_service_root_pda_internal(
        gateway_root_pda,
        gas_service_root_pda,
        &crate::id(),
    )
    .0
}

/// This function derives the address of the associated token account based on
/// the provided interchain token service root PDA, wallet, and mint. It also
/// performs a correctness check on the root PDA.
pub fn get_interchain_token_service_associated_token_account(
    its_root_pda: &Pubkey,
    wallet_account: &Pubkey,
    mint_account: &Pubkey,
    program_id: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    Ok(Pubkey::find_program_address(
        &[
            &its_root_pda.as_ref(),
            &wallet_account.as_ref(),
            &mint_account.as_ref(),
        ],
        program_id,
    ))
}

/// Derives the group ID for the operators permission group
/// The token ID is the only unique identifier for a token manager
/// therefore we use it as the group ID
/// https://github.com/axelarnetwork/interchain-token-service/blob/9f89c148259ca3337ed856415df6407f830ec4ea/contracts/utils/TokenManagerDeployer.sol#L33
pub fn get_operators_permission_group_id(
    token_id: &Bytes32,
    interchain_token_service_root_pda: &Pubkey,
) -> GroupId {
    GroupId::new(
        [
            &token_id.0,
            &interchain_token_service_root_pda.to_bytes(),
            "operators".as_bytes(),
        ]
        .concat(),
    )
}

/// Derives the group ID for the flow limiters permission group
/// The token ID is the only unique identifier for a token manager
/// therefore we use it as the group ID
/// https://github.com/axelarnetwork/interchain-token-service/blob/9f89c148259ca3337ed856415df6407f830ec4ea/contracts/utils/TokenManagerDeployer.sol#L33
pub fn get_flow_limiters_permission_group_id(
    token_id: &Bytes32,
    interchain_token_service_root_pda: &Pubkey,
) -> GroupId {
    GroupId::new(
        [
            &token_id.0,
            &interchain_token_service_root_pda.to_bytes(),
            "flow_limiters".as_bytes(),
        ]
        .concat(),
    )
}
