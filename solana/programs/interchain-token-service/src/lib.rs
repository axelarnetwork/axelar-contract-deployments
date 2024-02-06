#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
use borsh::{BorshDeserialize, BorshSerialize};
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// The type of token manager.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum TokenManagerType {
    /// Will simply transfer tokens from a user to itself or vice versa to
    /// initiate/fulfill cross-chain transfers.
    MintBurn,

    /// Works like the one above, but accounts for tokens that have a
    /// fee-on-transfer giving less tokens to be locked than what it actually
    /// transferred.
    MintBurnFrom,

    /// Will burn/mint tokens from/to the user to initiate/fulfill cross-chain
    /// transfers. Tokens used with this kind of TokenManager need to be
    /// properly permissioned to allow for this behaviour.
    LockUnlock,

    /// The same as the one above, but uses burnFrom instead of burn which is
    /// the standard for some tokens and typically requires an approval.
    LockUnlockFee,
}

solana_program::declare_id!("4ENH4KjzfcQwyXYr6SJdaF2nhMoGqdZJ2Hk5MoY9mU2G");

/// Checks that the supplied program ID is the correct one
pub fn check_program_account(program_id: &Pubkey) -> ProgramResult {
    if program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Derives interchain token service root PDA
pub(crate) fn get_interchain_token_service_root_pda_internal(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&gateway_root_pda.as_ref(), &gas_service_root_pda.as_ref()],
        program_id,
    )
}

/// Derives interchain token service root PDA
pub fn get_interchain_token_service_root_pda(
    gateway_root_pda: &Pubkey,
    gas_service_root_pda: &Pubkey,
) -> Pubkey {
    get_interchain_token_service_root_pda_internal(
        gateway_root_pda,
        gas_service_root_pda,
        &crate::id(),
    )
    .0
}

/// This function derives the address of the associated token account based on
/// the provided interchain token service root PDA, wallet, and mint. It also
/// performs a correctness check on the root PDA.
pub fn get_interchain_token_service_associated_token_account(
    its_root_pda: &Pubkey,
    wallet_account: &Pubkey,
    mint_account: &Pubkey,
    program_id: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    Ok(Pubkey::find_program_address(
        &[
            &its_root_pda.as_ref(),
            &wallet_account.as_ref(),
            &mint_account.as_ref(),
        ],
        program_id,
    ))
}
