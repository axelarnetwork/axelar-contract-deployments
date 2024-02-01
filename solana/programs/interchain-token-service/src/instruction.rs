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
    /// Initialize the InterchainTokenService program
    Initialize {},
    /// Execute a GMP payload
    Execute {
        /// GMP payload
        payload: Vec<u8>,
    },
}

/// Builds a `Setup` instruction for the `TokenManager` program.
///
/// # Returns
///
/// * `Instruction` - The `Setup` instruction for the `TokenManager` program.
///
/// # Errors
///
/// Will return `ProgramError` if the instruction data cannot be serialized.
#[allow(clippy::too_many_arguments)]
pub fn build_initialize_instruction(
    funder: &Pubkey,
    interchain_token_service_root_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainTokenServiceInstruction::Initialize {})?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*interchain_token_service_root_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(*gas_service_root_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
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
