//! Instruction types

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::ethers_core::abi::AbiEncode;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::id;

/// Instructions supported by the InterchainTokenService program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Execute a GMP payload
    Execute {
        /// GMP payload
        payload: Vec<u8>,
    },
}

/// Create `Execute` instruction
pub fn build_execute_instruction(
    funder: &Pubkey,
    incoming_accounts: &[AccountMeta],
    payload: impl AbiEncode,
) -> Result<Instruction, ProgramError> {
    let payload = payload.encode();
    let init_data = InterchainTokenServiceInstruction::Execute { payload };
    let data = to_vec(&init_data)?;

    let mut accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    accounts.extend_from_slice(incoming_accounts);

    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}
