//! Axelar Gateway program for the Solana blockchain
#![allow(stable_features)] // solana uses an old Rust compiler
#![feature(result_option_inspect)]

pub mod axelar_auth_weighted;
pub mod entrypoint;
pub mod error;
pub mod events;
pub mod instructions;
pub mod processor;
pub mod state;

pub use bytemuck;
// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::{Pubkey, PubkeyError};

solana_program::declare_id!("gtwgM94UYHwBh3g7rWi1tcpkgELxHQRLPpPHsaECW57");

/// Seed prefixes for different PDAs initialized by the Gateway
pub mod seed_prefixes {
    /// The seed prefix for deriving Gateway Config PDA
    pub const GATEWAY_SEED: &[u8] = b"gateway";
    /// The seed prefix for deriving VerifierSetTracker PDAs
    pub const VERIFIER_SET_TRACKER_SEED: &[u8] = b"ver-set-tracker";
    /// The seed prefix for deriving signature verification PDAs
    pub const SIGNATURE_VERIFICATION_SEED: &[u8] = b"gtw-sig-verif";
    /// The seed prefix for deriving incoming message PDAs
    pub const INCOMING_MESSAGE_SEED: &[u8] = b"incoming message";
}

/// Checks that the supplied program ID is the correct one
#[inline]
pub fn check_program_account(program_id: Pubkey) -> ProgramResult {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Get the root PDA and bump seed for the given program ID.
#[inline]
pub(crate) fn get_gateway_root_config_internal(program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed_prefixes::GATEWAY_SEED], program_id)
}

/// Get the root PDA and bump seed for the given program ID.
#[inline]
pub fn get_gateway_root_config_pda() -> (Pubkey, u8) {
    get_gateway_root_config_internal(&crate::ID)
}

/// Assert that the gateway PDA has been derived correctly
#[inline]
#[track_caller]
pub fn assert_valid_gateway_root_pda(
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::GATEWAY_SEED, &[bump]], &crate::ID)
            .expect("invalid bump for the root pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid Gateway Root PDA ");
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Get the incomeng message PDA & bump
#[inline]
pub fn get_incoming_message_pda(command_id: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, command_id],
        &crate::id(),
    )
}

/// Assert that the incomeing message PDA has been derived correctly
#[inline]
pub fn assert_valid_incoming_message_pda(
    command_id: &[u8],
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, command_id, &[bump]],
        &crate::ID,
    )
    .expect("invalid bump for the root pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid incoming message PDA ");
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Get the PDA and bump seed for a given verifier set hash.
/// This is used to calculate the PDA for VerifierSetTracker.
#[inline]
pub fn get_verifier_set_tracker_pda(
    hash: crate::state::verifier_set_tracker::VerifierSetHash,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::VERIFIER_SET_TRACKER_SEED, hash.as_slice()],
        &crate::id(),
    )
}

/// Assert that the verifier set tracker PDA has been derived correctly
#[inline]
pub fn assert_valid_verifier_set_tracker_pda(
    tracker: &crate::state::verifier_set_tracker::VerifierSetTracker,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::VERIFIER_SET_TRACKER_SEED,
            tracker.verifier_set_hash.as_slice(),
            &[tracker.bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the root pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid Verifier Set Root PDA ");
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Get the PDA and bump seed for a given payload hash.
#[inline]
pub fn get_signature_verification_pda(
    gateway_root_pda: &Pubkey,
    payload_merkle_root: &[u8; 32],
) -> (Pubkey, u8) {
    let (pubkey, bump) = Pubkey::find_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            gateway_root_pda.as_ref(),
            payload_merkle_root,
        ],
        &crate::ID,
    );
    (pubkey, bump)
}

/// Create the PDA for a given payload hash and bump.
#[inline]
pub fn create_signature_verification_pda(
    gateway_root_pda: &Pubkey,
    payload_merkle_root: &[u8; 32],
    bump: u8,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            gateway_root_pda.as_ref(),
            payload_merkle_root,
            &[bump],
        ],
        &crate::ID,
    )
}

/// Test that the bump from `get_signature_verification_pda` generates the same
/// public key when used with the same hash by
/// `create_signature_verification_pda`.
#[test]
fn test_get_and_create_signature_verification_pda_bump_reuse() {
    let gateway_root_pda = Pubkey::new_unique();
    let random_bytes = [43; 32];
    let (found_pda, bump) = get_signature_verification_pda(&gateway_root_pda, &random_bytes);
    let created_pda =
        create_signature_verification_pda(&gateway_root_pda, &random_bytes, bump).unwrap();
    assert_eq!(found_pda, created_pda);
}

#[test]
fn test_valid_gateway_root_pda_generation() {
    let (internal, bump_i) = get_gateway_root_config_internal(&crate::ID);
    assert_valid_gateway_root_pda(bump_i, &internal).unwrap();

    let (external, bump_e) = get_gateway_root_config_pda();
    assert_valid_gateway_root_pda(bump_e, &external).unwrap();

    assert_eq!(internal, external);
    assert_eq!(bump_i, bump_e);
}
