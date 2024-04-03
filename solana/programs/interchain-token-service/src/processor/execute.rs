mod deploy_interchain_token;
mod deploy_token_manager;
mod interchain_transfer;

use axelar_executable::validate_contract_call;
use axelar_message_primitives::AxelarExecutablePayload;
use interchain_token_transfer_gmp::ethers_core::abi::AbiDecode;
use interchain_token_transfer_gmp::GMPPayload;
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::{assert_root_its_derivation, Processor};
use crate::state::RootPDA;

impl Processor {
    /// This function is used to initialize the program.
    pub fn execute(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        payload: AxelarExecutablePayload,
    ) -> ProgramResult {
        validate_contract_call(program_id, accounts, &payload)?;

        let account_info_iter = &mut accounts.iter();
        let _gateway_approved_message_pda = next_account_info(account_info_iter)?;
        let _signing_pda = next_account_info(account_info_iter)?;
        let gateway_root_pda = next_account_info(account_info_iter)?;
        let _gateway_program_id = next_account_info(account_info_iter)?;

        let its_root_pda = next_account_info(account_info_iter)?;
        let gas_service_root_pda = next_account_info(account_info_iter)?;

        let root_pda = its_root_pda.check_initialized_pda::<RootPDA>(&crate::id())?;
        assert_root_its_derivation(
            gateway_root_pda,
            gas_service_root_pda,
            &root_pda,
            its_root_pda,
        )?;

        msg!("Executing GMP payload");
        let res = GMPPayload::decode(payload.payload_without_accounts.as_slice())
            .map_err(|_| ProgramError::InvalidInstructionData)?;

        match res {
            GMPPayload::InterchainTransfer(payload) => {
                msg!("GMPPayload::InterchainTransfer");
                Self::interchain_transfer(program_id, accounts, payload, &root_pda)
            }
            GMPPayload::DeployInterchainToken(payload) => {
                msg!("GMPPayload::DeployInterchainToken");
                Self::deploy_interchain_token(program_id, accounts, payload, &root_pda)
            }
            GMPPayload::DeployTokenManager(payload) => {
                msg!("GMPPayload::DeployTokenManager");
                Self::relayer_gmp_deploy_token_manager(program_id, accounts, payload, &root_pda)
            }
        }
    }
}
