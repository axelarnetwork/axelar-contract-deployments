//! Program state processor
mod deploy_interchain_token;
mod deploy_token_manager;
mod interchain_transfer;

use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::ethers_core::abi::AbiDecode;
use interchain_token_transfer_gmp::GMPPayload;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::instruction::InterchainTokenServiceInstruction;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = InterchainTokenServiceInstruction::try_from_slice(input)?;

        match instruction {
            InterchainTokenServiceInstruction::Execute { payload } => {
                let res = GMPPayload::decode(payload.as_slice())
                    .map_err(|_| ProgramError::InvalidInstructionData)?;

                match res {
                    GMPPayload::InterchainTransfer(payload) => {
                        Self::interchain_transfer(program_id, accounts, payload)
                    }
                    GMPPayload::DeployInterchainToken(payload) => {
                        Self::deploy_interchain_token(program_id, accounts, payload)
                    }
                    GMPPayload::DeployTokenManager(payload) => {
                        Self::deploy_token_manager(program_id, accounts, payload)
                    }
                }
            }
        }
    }
}
