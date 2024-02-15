use ethers_core::abi::{self, Token, Tokenizable};
use gateway::types::u256::U256;
use interchain_token_transfer_gmp::DeployInterchainTokenB;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hash;
use solana_program::pubkey::Pubkey;
use token_manager::TokenManagerType;

use super::Processor;
use crate::events::emit_interchain_token_id_claimed_event;

impl Processor {
    /// Used to deploy remote custom TokenManagers.
    ///
    /// At least the `gasValue` amount of native token must be passed to the
    /// function call. `gasValue` exists because this function can be
    /// part of a multicall involving multiple functions that could make remote
    /// contract calls.
    ///
    /// # Arguments
    ///
    /// * `program_id` - The program ID of the Solana program.
    /// * `accounts` - The accounts required for the transaction.
    /// * `salt` - The salt to be used during deployment.
    /// * `destination_chain` - The name of the chain to deploy the TokenManager
    ///   and standardized token to.
    /// * `token_manager_type` - The type of TokenManager to be deployed.
    /// * `params` - The params that will be used to initialize the
    ///   TokenManager.
    /// * `gas_value` - The amount of native tokens to be used to pay for gas
    ///   for the remote deployment.
    pub fn deploy_token_manager(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        salt: [u8; 32],
        destination_chain: Vec<u8>,
        token_manager_type: TokenManagerType,
        params: Vec<u8>,
        gas_value: U256,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?;

        emit_interchain_token_id_claimed_event(
            Self::interchain_token_id(sender.key, salt),
            (*sender.key).into(),
            salt,
        )

        // if (bytes(destinationChain).length == 0) {
        //     _deployTokenManager(tokenId, tokenManagerType, params);
        // } else {
        //     _deployRemoteTokenManager(tokenId, destinationChain, gasValue,
        // tokenManagerType, params); }
    }

    // Calculates the tokenId that would correspond to a link for a given
    // deployer with a specified salt.
    //
    // * `sender` - The address of the TokenManager deployer.
    // * `salt` - The salt that the deployer uses for the deployment.
    //
    // Returns the tokenId that the custom TokenManager would get (or has
    // gotten).
    fn interchain_token_id(sender: &Pubkey, salt: [u8; 32]) -> [u8; 32] {
        let sender = ethers_core::types::Bytes::from_iter(sender.as_ref().into_iter()).into_token();
        let salt = ethers_core::types::Bytes::from_iter(salt.as_ref().into_iter()).into_token();

        hash(&abi::encode(&[sender, salt])).to_bytes()
    }
}
