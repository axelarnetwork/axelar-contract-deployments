//! Axelar Gateway program for the Solana blockchain
pub mod entrypoint;
pub mod error;
pub mod events;
pub mod executable;
pub mod instructions;
pub mod processor;
pub mod state;
pub mod types;

pub use bytemuck;
pub use num_traits;
pub use program_utils::ensure_single_feature;
pub use program_utils::pda::BytemuckedPda;

// Export current sdk types for downstream users building with a different sdk
// version.
pub use solana_program;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::{Pubkey, PubkeyError};

use crate::error::GatewayError;
use crate::state::GatewayConfig;
use program_utils::pda::ValidPDA;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("gtwi5T9x6rTWPtuuz6DA7ia1VmH8bdazm9QfDdi6DVp");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("gtwqQzBirGUVdAUDt17WWYnVfPoydn9eAazDJb7gFUs");

#[cfg(feature = "testnet")]
solana_program::declare_id!("gtwShW9qgckMsZKijtRkNnyZHL4CU1BjZMftWiu7fGW");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("gtw1111111111111111111111111111111111111111");

/// Seed prefixes for different PDAs initialized by the Gateway
pub mod seed_prefixes {
    /// The seed prefix for deriving Gateway Config PDA
    pub const GATEWAY_SEED: &[u8] = b"gateway";
    /// The seed prefix for deriving `VerifierSetTracker` PDAs
    pub const VERIFIER_SET_TRACKER_SEED: &[u8] = b"ver-set-tracker";
    /// The seed prefix for deriving signature verification PDAs
    pub const SIGNATURE_VERIFICATION_SEED: &[u8] = b"gtw-sig-verif";
    /// The seed prefix for deriving call contract signature verification PDAs
    pub const CALL_CONTRACT_SIGNING_SEED: &[u8] = b"gtw-call-contract";
    /// The seed prefix for deriving incoming message PDAs
    pub const INCOMING_MESSAGE_SEED: &[u8] = b"incoming message";
    /// The seed prefix for deriving message payload PDAs
    pub const MESSAGE_PAYLOAD_SEED: &[u8] = b"message-payload";
    /// The seed prefix for deriving validate message signing PDAs
    /// This corresponds to the hardcoded value in `axelar_message_primitives::destination_program_id::DestinationProgramId::signing_pda`
    pub const VALIDATE_MESSAGE_SIGNING_SEED: &[u8] = b"gtw-validate-msg";
}

/// Checks that the supplied program ID is the correct one
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the provided program ID does not match this
/// program's ID.
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
#[must_use]
pub fn get_gateway_root_config_pda() -> (Pubkey, u8) {
    get_gateway_root_config_internal(&crate::ID)
}

/// Assert that the gateway PDA has been initialized and is valid
pub fn assert_initialized_and_valid_gateway_root_pda(
    gw_root_pda: &AccountInfo<'_>,
) -> Result<(), ProgramError> {
    gw_root_pda.check_initialized_pda_without_deserialization(&crate::ID)?;
    let gateway_data = gw_root_pda.try_borrow_data()?;
    let gateway_config =
        GatewayConfig::read(&gateway_data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
    assert_valid_gateway_root_pda(gateway_config.bump, gw_root_pda.key)
}

/// Assert that the gateway PDA has been derived correctly
///
/// # Panics
///
/// Panics if the bump seed produces an invalid program derived address.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the derived PDA does not match the expected pubkey.
#[inline]
#[track_caller]
fn assert_valid_gateway_root_pda(bump: u8, expected_pubkey: &Pubkey) -> Result<(), ProgramError> {
    let derived_pubkey =
        Pubkey::create_program_address(&[seed_prefixes::GATEWAY_SEED, &[bump]], &crate::ID)
            .expect("invalid bump for the root pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid Gateway Root PDA ");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Get the incoming message PDA & bump
#[inline]
#[must_use]
pub fn get_incoming_message_pda(command_id: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, command_id],
        &crate::ID,
    )
}

/// Creates the `IncomingMessage` PDA from a bump previously calculated
/// by [`get_incoming_message_pda`].
///
/// # Errors
///
/// Returns a [`PubkeyError`] if the derived address lies on the ed25519 curve and is therefore not
/// a valid program derived address.
#[inline]
pub fn create_incoming_message_pda(command_id: [u8; 32], bump: u8) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, &command_id, &[bump]],
        &crate::ID,
    )
}

/// Assert that the incoming message PDA has been derived correctly
///
/// # Panics
///
/// Panics if the bump seed produces an invalid program derived address.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the derived PDA does not match the expected pubkey.
#[inline]
#[track_caller]
pub fn assert_valid_incoming_message_pda(
    command_id: &[u8],
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[seed_prefixes::INCOMING_MESSAGE_SEED, command_id, &[bump]],
        &crate::ID,
    )
    .expect("invalid bump for the incoming message PDA");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid incoming message PDA ");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Assert that the message payload PDA has been derived correctly
///
/// # Errors
///
/// Returns [`ProgramError::InvalidSeeds`] if the derived PDA does not match the expected pubkey.
///
/// # Panics
///
/// Panics if the bump seed produces an invalid program derived address.
#[inline]
#[track_caller]
pub fn assert_valid_message_payload_pda(
    incoming_message_pda: Pubkey,
    payer: Pubkey,
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = create_message_payload_pda(incoming_message_pda, payer, bump)
        .expect("invalid bump for the message payload PDA");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid message payload PDA ");
        return Err(ProgramError::InvalidSeeds);
    }
    Ok(())
}

/// Get the PDA and bump seed for a given verifier set hash.
/// This is used to calculate the PDA for `VerifierSetTracker`.
#[inline]
#[must_use]
pub fn get_verifier_set_tracker_pda(
    hash: crate::state::verifier_set_tracker::VerifierSetHash,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::VERIFIER_SET_TRACKER_SEED, hash.as_slice()],
        &crate::ID,
    )
}

/// Assert that the verifier set tracker PDA has been derived correctly
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the derived PDA pubkey does not match the
/// expected pubkey.
///
/// # Panics
///
/// Panics if PDA creation fails due to an invalid bump seed.
#[inline]
#[track_caller]
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
    .expect("invalid bump for the verifier set tracker pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid Verifier Set Root PDA ");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Get the PDA and bump seed for a given payload hash.
#[inline]
#[must_use]
pub fn get_signature_verification_pda(
    payload_merkle_root: &[u8; 32],
    signing_verifier_set: &[u8; 32],
) -> (Pubkey, u8) {
    let (pubkey, bump) = Pubkey::find_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            payload_merkle_root,
            signing_verifier_set,
        ],
        &crate::ID,
    );
    (pubkey, bump)
}

/// Assert that the signature verification PDA has been derived correctly.
///
/// # Errors
///
/// Returns [`ProgramError::IncorrectProgramId`] if the derived PDA
/// pubkey does not match the expected pubkey.
///
/// # Panics
///
/// Panics if PDA creation fails due to an invalid bump seed.
#[inline]
#[track_caller]
pub fn assert_valid_signature_verification_pda(
    payload_merkle_root: &[u8; 32],
    signing_verifier_set: &[u8; 32],
    bump: u8,
    expected_pubkey: &Pubkey,
) -> Result<(), ProgramError> {
    let derived_pubkey = Pubkey::create_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            payload_merkle_root,
            signing_verifier_set,
            &[bump],
        ],
        &crate::ID,
    )
    .expect("invalid bump for the pda");
    if &derived_pubkey != expected_pubkey {
        solana_program::msg!("Error: Invalid Verifier Set Root PDA ");
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Create the PDA for a given payload hash and bump.
///
/// # Errors
///
/// Returns a [`PubkeyError`] if the derived address lies on the ed25519 curve and is therefore not
/// a valid program derived address.
#[inline]
pub fn create_signature_verification_pda(
    payload_merkle_root: &[u8; 32],
    signing_verifier_set: &[u8; 32],
    bump: u8,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::SIGNATURE_VERIFICATION_SEED,
            payload_merkle_root,
            signing_verifier_set,
            &[bump],
        ],
        &crate::ID,
    )
}

/// Create a new Signing PDA that is used for validating that a message has
/// reached the destination program.
#[inline]
#[must_use]
pub fn get_validate_message_signing_pda(
    destination_address: Pubkey,
    command_id: [u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED,
            command_id.as_ref(),
        ],
        &destination_address,
    )
}

/// Create a new Signing PDA that is used for validating that a message has
/// reached the destination program.
///
/// # Errors
///
/// Returns a [`PubkeyError`] if the derived address lies on the ed25519 curve and is therefore not
/// a valid program derived address when using the destination address as the program ID.
#[inline]
pub fn create_validate_message_signing_pda(
    destination_address: &Pubkey,
    signing_pda_bump: u8,
    command_id: &[u8; 32],
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::VALIDATE_MESSAGE_SIGNING_SEED,
            command_id,
            &[signing_pda_bump],
        ],
        destination_address,
    )
}

/// Create a new Signing PDA that is used for `CallContract` call by the source contract to authorize its call
#[inline]
#[must_use]
pub fn get_call_contract_signing_pda(source_program_id: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seed_prefixes::CALL_CONTRACT_SIGNING_SEED],
        &source_program_id,
    )
}

/// Create a new Signing PDA that is used for authorizing the source program to call `CallContract`
///
/// # Errors
///
/// Returns a [`PubkeyError`] if the derived address lies on the ed25519 curve and is therefore not
/// a valid program derived address when using the destination address as the program ID.
#[inline]
pub fn create_call_contract_signing_pda(
    source_program_id: Pubkey,
    signing_pda_bump: u8,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::CALL_CONTRACT_SIGNING_SEED,
            &[signing_pda_bump],
        ],
        &source_program_id,
    )
}

/// Finds the `MessagePayload` PDA.
///
/// This function is expensive and should not be used on-chain. Prefer
/// using [`create_message_payload_pda`] instead.
#[inline]
#[must_use]
pub fn find_message_payload_pda(incoming_message_pda: Pubkey, payer: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::MESSAGE_PAYLOAD_SEED,
            incoming_message_pda.as_ref(),
            payer.as_ref(),
        ],
        &crate::ID,
    )
}

/// Creates the `MessagePayload` PDA from a bump previously calculated
/// by [`find_message_payload_pda`].
///
/// # Errors
///
/// Returns a [`PubkeyError`] if the derived address lies on the ed25519 curve and is therefore not
/// a valid program derived address.
#[inline]
pub fn create_message_payload_pda(
    incoming_message_pda: Pubkey,
    payer: Pubkey,
    bump: u8,
) -> Result<Pubkey, PubkeyError> {
    Pubkey::create_program_address(
        &[
            seed_prefixes::MESSAGE_PAYLOAD_SEED,
            incoming_message_pda.as_ref(),
            payer.as_ref(),
            &[bump],
        ],
        &crate::ID,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that the bump from `get_signature_verification_pda` generates the same
    /// public key when used with the same hash by
    /// `create_signature_verification_pda`.
    #[test]
    fn test_get_and_create_signature_verification_pda_bump_reuse() {
        let payload_merkle_root = rand::random();
        let signing_verifier_set = rand::random();
        let (found_pda, bump) =
            get_signature_verification_pda(&payload_merkle_root, &signing_verifier_set);
        let created_pda =
            create_signature_verification_pda(&payload_merkle_root, &signing_verifier_set, bump)
                .unwrap();
        assert_eq!(found_pda, created_pda);
    }

    /// Test that the bump from `find_message_payload_pda` generates the same public key when
    /// used with the same inputs by `create_message_payload_pda`.
    #[test]
    fn test_find_and_create_message_payload_pda_bump_reuse() {
        let incoming_message_pda = Pubkey::new_unique();
        let payer = Pubkey::new_unique();
        let (found_pda, bump) = find_message_payload_pda(incoming_message_pda, payer);
        let created_pda = create_message_payload_pda(incoming_message_pda, payer, bump).unwrap();
        assert_eq!(found_pda, created_pda);
    }

    /// Test that the bump from `get_incoming_message_pda` generates the same public key when
    /// used with the same inputs by `create_incoming_message_pda`.
    #[test]
    fn test_get_and_create_incoming_message_pda_bump_reuse() {
        let command_id: [u8; 32] = rand::random();
        let (found_pda, bump) = get_incoming_message_pda(&command_id);
        let created_pda = create_incoming_message_pda(command_id, bump).unwrap();
        assert_eq!(found_pda, created_pda);
    }

    /// Test that the bump from `get_validate_message_signing_pda` generates the same public key when
    /// used with the same inputs by `create_validate_message_signing_pda`.
    #[test]
    fn test_get_and_create_validate_message_signing_pda_bump_reuse() {
        let destination_address = Pubkey::new_unique();
        let command_id: [u8; 32] = rand::random();
        let (found_pda, bump) = get_validate_message_signing_pda(destination_address, command_id);
        let created_pda =
            create_validate_message_signing_pda(&destination_address, bump, &command_id).unwrap();
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
}
