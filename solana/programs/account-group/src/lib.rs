#![deny(missing_docs)]

//! Interchain Address Tracker program for the Solana blockchain

mod entrypoint;
pub mod instruction;
pub mod processor;
pub mod state;
use instruction::GroupId;
pub use solana_program;
use solana_program::pubkey::Pubkey;

solana_program::declare_id!("7FsMLiW9cK1p4ivD9SGv1ZASDp2L3hQnwcDA3yXeDxsS");

/// Derives the permission group address and bump seed for the
/// given wallet address
pub(crate) fn get_permission_group_account_and_bump_seed_internal(
    id: &GroupId,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&id.to_bytes()], program_id)
}

/// Derives the permission address and bump seed for the given wallet
/// address
pub(crate) fn get_permission_account_and_bump_seed_internal(
    permission_group_pda: &Pubkey,
    permission_pda_owner: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            (permission_group_pda.as_ref()),
            (permission_pda_owner.as_ref()),
        ],
        program_id,
    )
}

/// Derives the permission group address for the given wallet
/// address
pub fn get_permission_group_account(op_id: &GroupId) -> Pubkey {
    get_permission_group_account_and_bump_seed_internal(op_id, &id()).0
}

/// Derives the permission address for the given wallet address
pub fn get_permission_account(
    permission_group_pda: &Pubkey,
    permission_pda_owner: &Pubkey,
) -> Pubkey {
    get_permission_account_and_bump_seed_internal(permission_group_pda, permission_pda_owner, &id())
        .0
}
