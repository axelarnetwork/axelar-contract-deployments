#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod error;
pub mod events;
pub mod instruction;
pub mod processor;
pub mod state;
use account_group::instruction::GroupId;
use axelar_message_primitives::command::U256 as GatewayU256;
use borsh::{BorshDeserialize, BorshSerialize};
use ethers_core::abi::{self, Tokenizable};
use ethers_core::types::U256 as EthersU256;
use interchain_token_transfer_gmp::Bytes32;
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hash;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Supported Metadata versions.
#[derive(Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = false)]
pub enum MetadataVersion {
    /// Metadata ContractCall. Used for gas payments.
    ContractCall = 0,
    /// Metadata ExpressCall. Used for gas payments.
    ExpressCall = 1,
}
/// Latest version of metadata that's supported.
pub const LATEST_METADATA_VERSION: u8 = 1;

solana_program::declare_id!("4ENH4KjzfcQwyXYr6SJdaF2nhMoGqdZJ2Hk5MoY9mU2G");

/// Checks that the supplied program ID is the correct one.
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

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

/// Calculates the tokenId that would correspond to a link for a given
/// deployer with a specified salt.
///
/// * `sender` - The address of the TokenManager deployer.
/// * `salt` - The salt that the deployer uses for the deployment.
///
/// Returns the tokenId that the custom TokenManager would get (or has
/// gotten).
pub fn interchain_token_id(sender: &Pubkey, salt: [u8; 32]) -> [u8; 32] {
    // INFO: this could be pre-calculated to save gas.
    let prefix = ethers_core::types::Bytes::from_iter(
        hash("its-interchain-token-id".as_bytes())
            .to_bytes()
            .as_ref()
            .iter(),
    )
    .into_token();
    let sender = ethers_core::types::Bytes::from_iter(sender.as_ref().iter()).into_token();
    let salt = ethers_core::types::Bytes::from_iter(salt.as_ref().iter()).into_token();

    hash(&abi::encode(&[prefix, sender, salt])).to_bytes()
}

/// Convert gateway u256 to ethers u256.
pub fn convert_gateway_u256_to_ethers_u256(ours: GatewayU256) -> EthersU256 {
    let bytes = ours.to_le_bytes();
    EthersU256::from_little_endian(&bytes)
}

/// Convert ethers u256 to gateway u256.
pub fn convert_ethers_u256_to_gateway_u256(ethers: EthersU256) -> GatewayU256 {
    let mut bytes = [0u8; 32];
    ethers.to_little_endian(&mut bytes);
    GatewayU256::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_gateway_u256_to_ethers() -> Result<()> {
        let bytes = [1u8; 32];
        let gateway = GatewayU256::from_le_bytes(bytes);
        EthersU256::from_little_endian(&bytes);
        assert_eq!(gateway.to_le_bytes(), bytes);
        Ok(())
    }

    #[test]
    fn test_ethers_u256_to_gateway() -> Result<()> {
        let bytes = [1u8; 32];
        EthersU256::from_little_endian(&bytes);
        let gateway = GatewayU256::from_le_bytes([1u8; 32]);
        assert_eq!(gateway.to_le_bytes(), bytes);
        Ok(())
    }
}
