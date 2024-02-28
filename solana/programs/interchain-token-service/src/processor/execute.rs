mod deploy_interchain_token;
mod deploy_token_manager;
mod interchain_transfer;

use gateway::accounts::GatewayApprovedMessage;
use interchain_token_transfer_gmp::ethers_core::abi::AbiDecode;
use interchain_token_transfer_gmp::GMPPayload;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{assert_root_its_derivation, Processor};
use crate::state::RootPDA;

impl Processor {
    /// This function is used to initialize the program.
    pub fn execute(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload: Vec<u8>,
    ) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let gateway_approved_message_pda = next_account_info(account_info_iter)?;
        let its_root_pda = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;

        let root_pda = its_root_pda.check_initialized_pda::<RootPDA>(&crate::id())?;
        assert_root_its_derivation(
            gateway_root_pda,
            gas_service_root_pda,
            &root_pda,
            its_root_pda,
        )?;

        let _approved_msg = gateway_approved_message_pda
            .check_initialized_pda::<GatewayApprovedMessage>(&gateway::id())?;
        invoke_signed(
            &gateway::instructions::validate_contract_call(
                gateway_approved_message_pda.key,
                its_root_pda.key,
            )?,
            &[gateway_approved_message_pda.clone(), its_root_pda.clone()],
            &[&[
                &gateway_root_pda.key.as_ref(),
                &gas_service_root_pda.key.as_ref(),
                &[root_pda.bump_seed],
            ]],
        )?;

        // TODO we need check if the payload hash is the same as the one in the gateway
        // approved message.      Otherwise someone could just send a different
        // payload and it would be executed.

        let res = GMPPayload::decode(payload.as_slice())
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match res {
            GMPPayload::InterchainTransfer(payload) => {
                Self::interchain_transfer(program_id, accounts, payload, &root_pda)
            }
            GMPPayload::DeployInterchainToken(payload) => {
                Self::deploy_interchain_token(program_id, accounts, payload, &root_pda)
            }
            GMPPayload::DeployTokenManager(payload) => {
                Self::relayer_gmp_deploy_token_manager(program_id, accounts, payload, &root_pda)
            }
        }
    }
}
