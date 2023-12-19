#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
pub use solana_program;
use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("7FsMLiW9cK1p4ivD9SGv1ZASDp2L3hQnwcDA3yXeDxsS");

/// Derives the operator group address and bump seed for the
/// given wallet address
pub(crate) fn get_operator_group_account_and_bump_seed_internal(
    id: &str,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    let chain_name = hash(id.as_bytes());
    Pubkey::find_program_address(&[&chain_name.to_bytes()], program_id)
}

/// Derives the operator address and bump seed for the given wallet
/// address
pub(crate) fn get_operator_address_account_and_bump_seed_internal(
    operator_group_pda: &Pubkey,
    operator: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[&operator.as_ref(), &operator_group_pda.as_ref()],
        program_id,
    )
}

/// Derives the operator group address for the given wallet
/// address
pub fn get_operator_group_account(op_id: &str) -> Pubkey {
    get_operator_group_account_and_bump_seed_internal(op_id, &id()).0
}

/// Derives the operator address for the given wallet address
pub fn get_operator_account(operator_group_pda: &Pubkey, operator: &Pubkey) -> Pubkey {
    get_operator_address_account_and_bump_seed_internal(operator_group_pda, operator, &id()).0
}
