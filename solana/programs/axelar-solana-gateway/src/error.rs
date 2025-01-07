//! Error types

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::ToPrimitive;
use solana_program::program_error::ProgramError;

const IRRECOVERABLE_ERROR: u32 = 500;

/// Errors that may be returned by the Gateway program.
///
/// The purpose of custom errors is mainly to aid the Relayer in understanding what actions to take.
/// This is because while some errors can be interpreted as "the argument / account / ix combo is unsuccessful",
/// some other errors can be interpreted as "this action has already been executed".
///
/// Because of this the errors are following the error numbers as follows:
/// Range: 0..500  | Action has already been completed by another actor. Relayer can interpret as "assume that this action completed successfully".
/// Range: 500..xx | Action cannot be completed with the provided arguments
#[repr(u32)]
#[derive(Clone, Debug, Eq, thiserror::Error, FromPrimitive, ToPrimitive, PartialEq)]
pub enum GatewayError {
    /// Verifier set has already been initialized.
    #[error("Verifier set already initialized")]
    VerifierSetAlreadyInitialised = 0,

    /// Used when someone tries to verify a signature that has already been verified.
    #[error("Slot has been previously verified")]
    SlotAlreadyVerified,

    /// The message has already been initialized.
    #[error("Message already initialized")]
    MessageAlreadyInitialised,

    /// The verification session PDA has already been initialized.
    #[error("Verification session PDA already initialized")]
    VerificationSessionPDAInitialised,

    /// The verifier set tracker PDA has already been initialized.
    #[error("Verifier set tracker PDA already initialized")]
    VerifierSetTrackerAlreadyInitialised,

    /// Used when a signature index is too high.
    #[error("Slot is out of bounds")]
    SlotIsOutOfBounds,

    /// Used when the internal digital signature verification fails.
    #[error("Digital signature verification failed")]
    InvalidDigitalSignature,

    /// Leaf node is not part of the Merkle root.
    #[error("Leaf node not part of Merkle root")]
    LeafNodeNotPartOfMerkleRoot,

    /// Used when the Merkle inclusion proof fails to verify against the given root.
    #[error("Signer is not a member of the active verifier set")]
    InvalidMerkleProof,

    /// Invalid destination address.
    #[error("Invalid destination address")]
    InvalidDestinationAddress,

    /// Message Payload PDA was already initialized.
    #[error("Message Payload PDA was already initialized")]
    MessagePayloadAlreadyInitialized,

    /// Message Payload has already been committed.
    #[error("Message Payload has already been committed")]
    MessagePayloadAlreadyCommitted,

    /// Error indicating an underflow occurred during epoch calculation.
    #[error("Epoch calculation resulted in an underflow")]
    // --- NOTICE ---
    // this bumps the error representation to start at 500
    // Any error after this point is deemed irrecoverable
    EpochCalculationOverflow = IRRECOVERABLE_ERROR,

    /// Error indicating the provided verifier set is too old.
    #[error("Verifier set too old")]
    VerifierSetTooOld,

    /// Data length mismatch when trying to read bytemucked data.
    #[error("Invalid bytemucked data length")]
    BytemuckDataLenInvalid,

    /// The signing session is not valid.
    #[error("Signing session not valid")]
    SigningSessionNotValid,

    /// Invalid verification session PDA.
    #[error("Invalid verification session PDA")]
    InvalidVerificationSessionPDA,

    /// Invalid verifier set tracker provided.
    #[error("Invalid verifier set tracker provided")]
    InvalidVerifierSetTrackerProvided,

    /// Proof not signed by the latest verifier set.
    #[error("Proof not signed by latest verifier set")]
    ProofNotSignedByLatestVerifierSet,

    /// Rotation cooldown not completed.
    #[error("Rotation cooldown not done")]
    RotationCooldownNotDone,

    /// Invalid program data derivation.
    #[error("Invalid program data derivation")]
    InvalidProgramDataDerivation,

    /// Invalid loader content.
    #[error("Invalid loader content")]
    InvalidLoaderContent,

    /// Invalid loader state.
    #[error("Invalid loader state")]
    InvalidLoaderState,

    /// Operator or upgrade authority must be a signer.
    #[error("Operator or upgrade authority must be signer")]
    OperatorOrUpgradeAuthorityMustBeSigner,

    /// Invalid operator or authority account.
    #[error("Invalid operator or authority account")]
    InvalidOperatorOrAuthorityAccount,

    /// Message has not been approved.
    #[error("Message not approved")]
    MessageNotApproved,

    /// Message has been tampered with.
    #[error("Message has been tampered with")]
    MessageHasBeenTamperedWith,

    /// Invalid signing PDA.
    #[error("Invalid signing PDA")]
    InvalidSigningPDA,

    /// Caller is not a signer.
    #[error("Caller not signer")]
    CallerNotSigner,
}

impl GatewayError {
    /// This is a utility function for the relayer when it's exepcting the error for an unsuccessful transaction.
    pub fn should_relayer_proceed(&self) -> bool {
        let Some(error_num) = self.to_u32() else {
            return false;
        };
        error_num < IRRECOVERABLE_ERROR
    }
}

impl From<GatewayError> for ProgramError {
    fn from(e: GatewayError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use num_traits::FromPrimitive;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_should_relayer_proceed() {
        let errors_to_proceed = (0..IRRECOVERABLE_ERROR)
            .map_while(GatewayError::from_u32)
            .collect_vec();
        let errors_to_not_proceed = (IRRECOVERABLE_ERROR..u32::MAX)
            .map_while(GatewayError::from_u32)
            .collect_vec();

        // confidence check that we derived the errors correctly
        assert_eq!(errors_to_proceed.len(), 12);
        assert_eq!(errors_to_not_proceed.len(), 17);

        // Errors that should cause the relayer to proceed (error numbers < 500)
        for error in errors_to_proceed {
            assert!(
                error.should_relayer_proceed(),
                "Error {:?} (code {}) should cause relayer to proceed",
                error,
                error.to_u32().unwrap()
            );
        }

        // Errors that should NOT cause the relayer to proceed (error numbers >= 500)
        for error in errors_to_not_proceed {
            assert!(
                !error.should_relayer_proceed(),
                "Error {:?} (code {}) should cause relayer NOT to proceed",
                error,
                error.to_u32().unwrap()
            );
        }
    }
}
