//! Axelar Gateway program for the Solana blockchain

pub mod axelar_auth_weighted;
pub mod commands;
mod entrypoint;
pub mod error;
pub mod events;
pub mod instructions;
pub mod processor;
pub mod state;

use axelar_rkyv_encoding::hasher::solana::SolanaKeccak256Hasher;
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
    pub const GATEWAY_SEED: &[u8; 7] = b"gateway";
    /// The seed prefix for deriving VerifierSetTracker PDAs
    pub const VERIFIER_SET_TRACKER_SEED: &[u8; 15] = b"ver-set-tracker";
    /// The seed prefix for deriving ExecuteData PDAs
    pub const EXECUTE_DATA_SEED: &[u8; 13] = b"gtw-exec-data";
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
pub fn assert_valid_gateway_root_pda(bump: u8, expected_pubkey: &Pubkey) {
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::GATEWAY_SEED, &[bump]], &crate::ID)
            .expect("invalid bump for the root pda");

    assert_eq!(&derived_pubkey, expected_pubkey, "invalid gateway root pda");
}

/// Get the PDA and bump seed for a given verifier set hash.
/// This is used to calculate the PDA for VerifierSetTracker.
#[inline]
pub fn get_verifier_set_tracker_pda(
    program_id: &Pubkey,
    hash: crate::state::verifier_set_tracker::VerifierSetHash,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::VERIFIER_SET_TRACKER_SEED, hash.as_slice()],
        program_id,
    )
}

/// Assert that the verifier set tracker PDA has been derived correctly
#[inline]
pub fn assert_valid_verifier_set_tracker_pda(
    tracker: &crate::state::verifier_set_tracker::VerifierSetTracker,
    expected_pubkey: &Pubkey,
) {
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::VERIFIER_SET_TRACKER_SEED,
            tracker.verifier_set_hash.as_slice(),
            &[tracker.bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the root pda");

    assert_eq!(&derived_pubkey, expected_pubkey, "invalid gateway root pda");
}

/// Get the PDA and bump seed for a given execute data hash.
#[inline]
pub fn get_execute_data_pda(
    gateway_root_pda: &Pubkey,
    hash: &crate::state::execute_data::ExecuteDataHash,
) -> (Pubkey, u8) {
    let (pubkey, bump) = Pubkey::find_program_address(
        &[
            seed_prefixes::EXECUTE_DATA_SEED,
            gateway_root_pda.as_ref(),
            hash,
        ],
        &crate::ID,
    );
    (pubkey, bump)
}

/// Create the PDA for a given execute data hash and bump.
#[inline]
pub fn create_execute_data_pda(
    gateway_root_pda: &Pubkey,
    hash: &crate::state::execute_data::ExecuteDataHash,
    bump: u8,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::EXECUTE_DATA_SEED,
            gateway_root_pda.as_ref(),
            hash,
            &[bump],
        ],
        &crate::ID,
    )
}

/// Asserts that the PDA for this account is valid.
#[inline]
pub fn assert_valid_execute_data_pda<T>(
    execute_data: &crate::state::execute_data::GatewayExecuteData<T>,
    gateway_root_pda: &Pubkey,
    expected_pubkey: &Pubkey,
) where
    T: crate::state::execute_data::ExecuteDataVariant,
{
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::EXECUTE_DATA_SEED,
            gateway_root_pda.as_ref(),
            &execute_data.hash_decoded_contents(),
            &[execute_data.bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the execute data pda");

    assert_eq!(
        &derived_pubkey, expected_pubkey,
        "invalid pda for the gateway execute data account"
    );
}

/// Provides abstraction for the hashing mechanism.
pub fn hasher_impl() -> SolanaKeccak256Hasher<'static> {
    SolanaKeccak256Hasher::default()
}

/// Test that the bump from `get_execute_data_pda` generates the same public key
/// when used with the same hash by `create_execute_data_pda`.
#[test]
fn test_get_and_create_execute_data_pda_bump_reuse() {
    let gateway_root_pda = Pubkey::new_unique();
    let random_bytes = Pubkey::new_unique().to_bytes();
    let (found_pda, bump) = get_execute_data_pda(&gateway_root_pda, &random_bytes);
    let created_pda = create_execute_data_pda(&gateway_root_pda, &random_bytes, bump).unwrap();
    assert_eq!(found_pda, created_pda);
}
