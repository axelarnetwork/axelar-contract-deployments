//! Instruction types

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
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
    ///   0. `[writeable,signer]` Funding account, pays for the chain account
    ///      creation
    ///   1. `[writable]` The account to initialize where we store the chain
    ///      name & other data.
    ///   2. `[signer]` The to-be owner account that is used for derivation of
    ///      the associated chain address (can be another PDA or a wallet
    ///      account)
    ///   3. `[]` The system program
    CreateRegisteredChain {
        /// Chain name of the remote chain
        chain_name: String,
    },
    /// Sets the trusted address and its hash for a remote chain.
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable,signer]` Funding account, pays for the chain account
    ///      creation
    ///   1. `[]` The associated chain account
    ///   2. `[signer]` The owner account of the associated chain account
    ///   3. `[writable]` The associated trusted address account where the data
    ///      will be stored
    ///   4. `[]` The system program
    SetTrustedAddress {
        /// Chain name of the remote chain
        chain_name: String,
        /// the string representation of the trusted address
        address: String,
    },
}

/// Create `CreateRegisteredChain` instruction
pub fn build_create_registered_chain_instruction(
    funder: &Pubkey,
    associated_chain_account: &Pubkey,
    owner: &Pubkey,
    chain_name: String,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainAddressTrackerInstruction::CreateRegisteredChain { chain_name })?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new(*associated_chain_account, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}

/// Create `SetTrustedAddress` instruction
pub fn build_set_trusted_address_instruction(
    funder: &Pubkey,
    associated_chain_account: &Pubkey,
    associated_trusted_address_account: &Pubkey,
    owner: &Pubkey,
    trusted_chain_name: String,
    trusted_chain_address: String,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&InterchainAddressTrackerInstruction::SetTrustedAddress {
        chain_name: trusted_chain_name,
        address: trusted_chain_address,
    })?;

    let accounts = vec![
        AccountMeta::new(*funder, true),
        AccountMeta::new_readonly(*associated_chain_account, false),
        AccountMeta::new_readonly(*owner, true),
        AccountMeta::new(*associated_trusted_address_account, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    Ok(Instruction {
        program_id: id(),
        accounts,
        data,
    })
}
