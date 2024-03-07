use ethers_core::abi::AbiEncode;
use interchain_address_tracker::state::RegisteredTrustedAddressAccount;
use interchain_token_transfer_gmp::DeployInterchainToken;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::invoke;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::InterchainTokenServiceError;
use crate::events::emit_interchain_token_deployment_started_event;
use crate::{interchain_token_id, Bytes32, ProgramError};

impl Processor {
    /// Used to deploy remote interchain tokens.
    #[allow(clippy::too_many_arguments)]
    pub fn deploy_remote_interchain_token(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        salt: [u8; 32],
        destination_chain: Vec<u8>,
        name: String,
        symbol: String,
        decimals: u8,
        minter: Vec<u8>,
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
        let token_id = interchain_token_id(sender.key, salt);

        let associated_trusted_address_data = RegisteredTrustedAddressAccount::unpack_from_slice(
            &associated_trusted_address.try_borrow_mut_data()?,
        )?;
        let destination_address = associated_trusted_address_data.address.into_bytes();

        assert!(sender.is_signer);

        emit_interchain_token_deployment_started_event(
            token_id,
            name.clone(),
            symbol.clone(),
            decimals,
            minter.clone(),
            destination_chain.clone(),
        )?;

        let payload = DeployInterchainToken {
            token_id: Bytes32(token_id),
            name,
            symbol,
            decimals,
            minter,
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
