//! Instruction types

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::id;

/// Instructions supported by the InterchainAddressTracker program.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum InterchainAddressTrackerInstruction {
    /// Initialize a new InterchainAddressTracker.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the chain account creation
    ///   1. `[writable]` The account to initialize where we store the chain name & other data.
    ///   1. `[]` The account that is used for derivation of the associated chain address (can be another PDA or a wallet account)
    ///   2. `[]` The system program
    CreateRegisteredChain {
        /// Chain name of the remote chain
        chain_name: String,
    },
}

/// Create `CreateRegisteredChain` instruction
pub fn build_create_registered_chain_instruction(
    funder: &Pubkey,
    associated_chain_account: &Pubkey,
    wallet_account: &Pubkey,
    chain_name: String,
) -> Result<Instruction, ProgramError> {
    let init_data = InterchainAddressTrackerInstruction::CreateRegisteredChain { chain_name };
    let data = init_data.try_to_vec()?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*associated_chain_account, false),
        AccountMeta::new_readonly(*wallet_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}
