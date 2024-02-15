use ethers_core::abi::{self, Tokenizable};
use gateway::types::u256::U256;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hash;
use solana_program::pubkey::Pubkey;
use token_manager::TokenManagerType;

use super::Processor;
use crate::events::{
    emit_interchain_token_id_claimed_event, emit_token_manager_deployment_started_event,
};
use crate::PREFIX_INTERCHAIN_TOKEN_ID;

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
    pub fn deploy_remote_token_manager(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        salt: [u8; 32],
        destination_chain: Vec<u8>,
        token_manager_type: TokenManagerType,
        params: Vec<u8>,
        _gas_value: U256,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?;
        let token_id = Self::interchain_token_id(sender.key, salt);

        emit_interchain_token_id_claimed_event(token_id, (*sender.key).into(), salt)?;

        // TODO: validTokenManagerAddress(tokenId);
        // https://github.com/axelarnetwork/interchain-token-service/blob/566e8504fe35ed63ae6c063dd8fd40a41fabc0c7/contracts/InterchainTokenService.sol#L906

        emit_token_manager_deployment_started_event(
            token_id,
            destination_chain,
            token_manager_type,
            params,
        )?;

        ProgramResult::Ok(())

        //     bytes memory payload =
        // abi.encode(MESSAGE_TYPE_DEPLOY_TOKEN_MANAGER,
        // tokenId, tokenManagerType, params);

        //     _callContract(destinationChain, payload,
        // MetadataVersion.CONTRACT_CALL, gasValue); }
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
        // INFO: this could be pre-calculated to save gas.
        let prefix = ethers_core::types::Bytes::from_iter(
            hash(PREFIX_INTERCHAIN_TOKEN_ID.as_bytes())
                .to_bytes()
                .as_ref()
                .iter(),
        )
        .into_token();
        let sender = ethers_core::types::Bytes::from_iter(sender.as_ref().iter()).into_token();
        let salt = ethers_core::types::Bytes::from_iter(salt.as_ref().iter()).into_token();

        hash(&abi::encode(&[prefix, sender, salt])).to_bytes()
    }
}
