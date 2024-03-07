use ethers_core::abi::AbiEncode;
use ethers_core::types::U256;
use interchain_address_tracker::state::RegisteredTrustedAddressAccount;
use interchain_token_transfer_gmp::DeployTokenManager;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::invoke;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use token_manager::TokenManagerType;

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::events::{
    emit_interchain_token_id_claimed_event, emit_token_manager_deployment_started_event,
};
use crate::{interchain_token_id, Bytes32, ProgramError};

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
    /// * `gas_value` / `fees` - The amount of native tokens to be used to pay
    ///   for gas for the remote deployment.
    pub fn deploy_remote_token_manager(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        salt: [u8; 32],
        destination_chain: Vec<u8>,
        token_manager_type: TokenManagerType,
        params: Vec<u8>,
        fees: u64,
    ) -> Result<(), ProgramError> {
        if destination_chain.is_empty() {
            return Err(InterchainTokenServiceError::UntrustedChain.into());
        }

        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let gas_service = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;
        let associated_trusted_address = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;
        let _gateway_program = next_account_info(account_info_iter)?;
        let token_id = interchain_token_id(sender.key, salt);

        let associated_trusted_address_data = RegisteredTrustedAddressAccount::unpack_from_slice(
            &associated_trusted_address.try_borrow_mut_data()?,
        )?;
        let destination_address = associated_trusted_address_data.address.into_bytes();

        assert!(sender.is_signer);

        emit_interchain_token_id_claimed_event(token_id, *sender.key, salt)?;
        emit_token_manager_deployment_started_event(
            token_id,
            destination_chain.clone(),
            token_manager_type.clone(),
            params.clone(),
        )?;

        let payload = DeployTokenManager {
            token_id: Bytes32(token_id),
            token_manager_type: U256::from(token_manager_type as u8),
            params,
        }
        .encode();

        if fees > 0_u64 {
            invoke(
                &gas_service::instruction::create_pay_native_gas_for_contract_call_ix(
                    *sender.key,
                    *sender.key,
                    destination_chain.clone(),
                    destination_address.clone(),
                    payload.clone(),
                    fees,
                )?,
                &[
                    sender.clone(),
                    gas_service_root_pda.clone(),
                    gas_service.clone(),
                    system_program.clone(),
                ],
            )?;
        }

        invoke(
            &gateway::instructions::call_contract(
                *gateway_root_pda.key,
                *sender.key,
                destination_chain,
                destination_address,
                payload,
            )?,
            &[sender.clone(), gateway_root_pda.clone()],
        )?;

        Ok(())
    }
}
