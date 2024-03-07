use ethers_core;
use ethers_core::abi::AbiEncode;
use interchain_token_transfer_gmp::{Bytes32, InterchainTransfer};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::program::invoke;
use solana_program::pubkey::Pubkey;
use token_manager::instruction::FlowToAdd;
use token_manager::TokenManagerType;

use super::Processor;
use crate::events::emit_interchain_transfer_event;
use crate::{instruction, MetadataVersion, ProgramError};

impl Processor {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn remote_interchain_transfer(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_id: [u8; 32],
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        amount: u64,
        data: Vec<u8>,
        metadata_version: MetadataVersion,
        symbol: Vec<u8>,
        token_manager_type: TokenManagerType,
    ) -> Result<(), ProgramError> {
        match token_manager_type {
            TokenManagerType::MintBurn => Self::remote_interchain_transfer_mint_burn(
                program_id,
                accounts,
                token_id,
                destination_chain,
                destination_address,
                amount,
                data,
                metadata_version,
                symbol,
            ),
            TokenManagerType::MintBurnFrom => todo!(),
            TokenManagerType::LockUnlock => Self::remote_interchain_transfer_lock_unlock(
                program_id,
                accounts,
                token_id,
                destination_chain,
                destination_address,
                amount,
                data,
                metadata_version,
                symbol,
            ),
            TokenManagerType::LockUnlockFee => todo!(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn remote_interchain_transfer_mint_burn(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_id: [u8; 32],
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        amount: u64,
        data: Vec<u8>,
        _metadata_version: MetadataVersion,
        symbol: Vec<u8>,
    ) -> Result<(), ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?;
        assert!(sender.is_signer);

        // take token
        let interchain_token_service_root_pda = next_account_info(account_info_iter)?;
        let owner_of_its_ata_for_user_tokens_pda = next_account_info(account_info_iter)?;
        let its_ata_for_user_tokens_pda = next_account_info(account_info_iter)?;
        let mint_account_pda = next_account_info(account_info_iter)?;
        let delegate_authority = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;
        // add flow
        let token_manager_pda = next_account_info(account_info_iter)?;
        let token_manager_flow_pda = next_account_info(account_info_iter)?;
        let flow_limiter_group_pda = next_account_info(account_info_iter)?;
        let flow_limiter_pda = next_account_info(account_info_iter)?;
        let flow_limiter = next_account_info(account_info_iter)?;
        let permission_group_pda = next_account_info(account_info_iter)?;
        let service_group_pda = next_account_info(account_info_iter)?;
        // our programs
        let _interchain_token_service_program = next_account_info(account_info_iter)?;
        let _token_manager_program = next_account_info(account_info_iter)?;
        let _gateway_program = next_account_info(account_info_iter)?;
        // system programs
        let spl_token_program = next_account_info(account_info_iter)?;
        let _spl_associated_token_account = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        invoke(
            &instruction::build_take_token_mint_burn_instruction(
                amount,
                sender.key,
                interchain_token_service_root_pda.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
                delegate_authority.key,
                gateway_root_pda.key,
                gas_service_root_pda.key,
            )?,
            &[
                sender.clone(),
                interchain_token_service_root_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                delegate_authority.clone(),
                gateway_root_pda.clone(),
                gas_service_root_pda.clone(),
                spl_token_program.clone(),
            ],
        )?;

        invoke(
            &token_manager::instruction::build_add_flow_instruction(
                sender.key,
                token_manager_pda.key,
                token_manager_flow_pda.key,
                flow_limiter_group_pda.key,
                flow_limiter_pda.key,
                flow_limiter.key,
                permission_group_pda.key,
                service_group_pda.key,
                FlowToAdd {
                    add_flow_in: 0,
                    add_flow_out: amount,
                },
            )?,
            &[
                sender.clone(),
                token_manager_pda.clone(),
                token_manager_flow_pda.clone(),
                flow_limiter_group_pda.clone(),
                flow_limiter_pda.clone(),
                flow_limiter.clone(),
                permission_group_pda.clone(),
                service_group_pda.clone(),
                system_program.clone(),
            ],
        )?;

        Self::transmit_interchain_transfer(
            sender,
            gateway_root_pda,
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            symbol,
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn remote_interchain_transfer_lock_unlock(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        token_id: [u8; 32],
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        amount: u64,
        data: Vec<u8>,
        _metadata_version: MetadataVersion,
        symbol: Vec<u8>,
    ) -> Result<(), ProgramError> {
        let account_info_iter = &mut accounts.iter();
        let sender = next_account_info(account_info_iter)?;
        assert!(sender.is_signer);

        // take token
        let interchain_token_service_root_pda = next_account_info(account_info_iter)?;
        let token_manager_ata_pda = next_account_info(account_info_iter)?;
        let owner_of_its_ata_for_user_tokens_pda = next_account_info(account_info_iter)?;
        let its_ata_for_user_tokens_pda = next_account_info(account_info_iter)?;
        let mint_account_pda = next_account_info(account_info_iter)?;
        let destination = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;
        // add flow
        let token_manager_pda = next_account_info(account_info_iter)?;
        let token_manager_flow_pda = next_account_info(account_info_iter)?;
        let flow_limiter_group_pda = next_account_info(account_info_iter)?;
        let flow_limiter_pda = next_account_info(account_info_iter)?;
        let flow_limiter = next_account_info(account_info_iter)?;
        let permission_group_pda = next_account_info(account_info_iter)?;
        let service_group_pda = next_account_info(account_info_iter)?;
        // our programs
        let _interchain_token_service_program = next_account_info(account_info_iter)?;
        let _token_manager_program = next_account_info(account_info_iter)?;
        let _gateway_program = next_account_info(account_info_iter)?;
        // system programs
        let spl_token_program = next_account_info(account_info_iter)?;
        let _spl_associated_token_account = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        invoke(
            &instruction::build_take_token_lock_unlock_instruction(
                amount,
                sender.key,
                interchain_token_service_root_pda.key,
                token_manager_ata_pda.key,
                owner_of_its_ata_for_user_tokens_pda.key,
                its_ata_for_user_tokens_pda.key,
                mint_account_pda.key,
                destination.key,
                gateway_root_pda.key,
                gas_service_root_pda.key,
            )?,
            &[
                sender.clone(),
                interchain_token_service_root_pda.clone(),
                token_manager_ata_pda.clone(),
                owner_of_its_ata_for_user_tokens_pda.clone(),
                its_ata_for_user_tokens_pda.clone(),
                mint_account_pda.clone(),
                destination.clone(),
                gateway_root_pda.clone(),
                gas_service_root_pda.clone(),
                spl_token_program.clone(),
            ],
        )?;

        // TODO: MetadataVersion / is it needed?
        // https://github.com/axelarnetwork/interchain-token-service/blob/0977738a1d7df5551cb3bd2e18f13c0e09944ff2/contracts/InterchainTokenService.sol#L468

        invoke(
            &token_manager::instruction::build_add_flow_instruction(
                sender.key,
                token_manager_pda.key,
                token_manager_flow_pda.key,
                flow_limiter_group_pda.key,
                flow_limiter_pda.key,
                flow_limiter.key,
                permission_group_pda.key,
                service_group_pda.key,
                FlowToAdd {
                    add_flow_in: 0,
                    add_flow_out: amount,
                },
            )?,
            &[
                sender.clone(),
                token_manager_pda.clone(),
                token_manager_flow_pda.clone(),
                flow_limiter_group_pda.clone(),
                flow_limiter_pda.clone(),
                flow_limiter.clone(),
                permission_group_pda.clone(),
                service_group_pda.clone(),
                system_program.clone(),
            ],
        )?;

        Self::transmit_interchain_transfer(
            sender,
            gateway_root_pda,
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            symbol,
        )?;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn transmit_interchain_transfer<'a, 'b>(
        sender: &'a AccountInfo<'b>,
        gateway_root_pda: &'a AccountInfo<'b>,
        token_id: [u8; 32],
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        amount: u64,
        data: Vec<u8>,
        symbol: Vec<u8>,
    ) -> Result<(), ProgramError> {
        emit_interchain_transfer_event(
            token_id,
            sender.key.to_bytes().to_vec(),
            destination_chain.clone(),
            destination_address.clone(),
            amount,
            data.clone(),
        )?;

        let payload = InterchainTransfer {
            token_id: Bytes32(token_id),
            source_address: sender.key.to_bytes().to_vec(),
            destination_address: destination_address.clone(),
            amount: ethers_core::types::U256::from_little_endian(&amount.to_le_bytes()),
            data,
        }
        .encode();

        if !symbol.is_empty() {
            // INFO: Ignored as per client request.
            // _callContractWithToken(destinationChain, payload, symbol,
            //     // amount, metadataVersion, gasValue);
        } else {
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
        }

        Ok(())
    }
}
