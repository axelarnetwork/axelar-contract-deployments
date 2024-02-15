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
    /// Some description here.
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

    fn interchain_token_id(sender: &Pubkey, salt: [u8; 32]) -> [u8; 32] {
        let sender = ethers_core::types::Bytes::from_iter(sender.as_ref().into_iter()).into_token();
        let salt = ethers_core::types::Bytes::from_iter(salt.as_ref().into_iter()).into_token();

        hash(&abi::encode(&[sender, salt])).to_bytes()
    }
}
