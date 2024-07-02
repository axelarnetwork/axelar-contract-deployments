use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use crate::types::{
    ArchivedPublicKey, ArchivedSignature, EcdsaPubkey, EcdsaRecoverableSignature, Ed25519Pubkey,
    Ed25519Signature, PublicKey, Signature, U256,
};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct WeightedSignature {
    pub(crate) pubkey: PublicKey,
    pub(crate) signature: Signature,
    pub(crate) weight: U256,
}

impl WeightedSignature {
    pub fn new(pubkey: PublicKey, signature: Signature, weight: U256) -> Self {
        Self {
            pubkey,
            signature,
            weight,
        }
    }
}

impl ArchivedWeightedSignature {
    pub(crate) fn verify(&self, message: &[u8; 32]) -> Result<(), SignatureVerificationError> {
        match (&self.signature, &self.pubkey) {
            (
                ArchivedSignature::EcdsaRecoverable(signature_bytes),
                ArchivedPublicKey::Ecdsa(public_key_bytes),
            ) => Self::verify_ecdsa(signature_bytes, public_key_bytes, message),
            (
                ArchivedSignature::Ed25519(signature_bytes),
                ArchivedPublicKey::Ed25519(public_key_bytes),
            ) => Self::verify_ed25519(signature_bytes, public_key_bytes, message),
            _ => Err(SignatureVerificationError::MismatchedPublicKeyAndSignatureScheme),
        }
    }

    fn verify_ecdsa(
        signature_bytes: &EcdsaRecoverableSignature,
        public_key_bytes: &EcdsaPubkey,
        message: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        use k256::ecdsa;
        use k256::ecdsa::signature::Verifier;

        // Unwrap: we use the right slice size so this never panics.
        let signature = k256::ecdsa::Signature::from_slice(&signature_bytes[0..64]).unwrap();

        // Recover the public key
        let recovery_id = {
            let recovery_id_byte = signature_bytes[64];
            ecdsa::RecoveryId::from_byte(recovery_id_byte).ok_or(
                SignatureVerificationError::InvalidRecoveryId(recovery_id_byte),
            )?
        };
        let recovered_public_key =
            ecdsa::VerifyingKey::recover_from_msg(message, &signature, recovery_id)
                .map_err(SignatureVerificationError::PublicKeyRecovery)?;

        // Double check: recovered public key is equivalent to the provided public key.
        let provided_public_key = ecdsa::VerifyingKey::from_sec1_bytes(public_key_bytes)
            .map_err(SignatureVerificationError::InvalidEcdsaPublicKeyBytes)?;
        if provided_public_key != recovered_public_key {
            return Err(SignatureVerificationError::PublicKeyRecoveryMismatch {
                recovered: Box::new(recovered_public_key),
                provided: Box::new(provided_public_key),
            });
        }
        recovered_public_key
            .verify(message, &signature)
            .map_err(SignatureVerificationError::EcdsaVerificationFailed)
    }

    fn verify_ed25519(
        signature_bytes: &Ed25519Signature,
        public_key_bytes: &Ed25519Pubkey,
        message: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let signature = Signature::from_bytes(signature_bytes);
        let public_key = VerifyingKey::from_bytes(public_key_bytes)
            .map_err(SignatureVerificationError::InvalidEd25519PublicKeyBytes)?;
        public_key
            .verify(message, &signature)
            .map_err(SignatureVerificationError::Ed25519VerificationFailed)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum SignatureVerificationError {
    #[error("Signature and public key schemes are different")]
    MismatchedPublicKeyAndSignatureScheme,
    #[error("Invalid ECDSA recovery byte: {0}")]
    InvalidRecoveryId(u8),
    #[error("Failed to recover ECDSA public key : {0}")]
    PublicKeyRecovery(#[source] k256::ecdsa::Error),
    #[error("ECDSA Signature verification failed : {0}")]
    EcdsaVerificationFailed(#[source] k256::ecdsa::Error),
    /// Fields are boxed to reduce the amount of stack space reserved.
    #[error("Recovered public key is different than provided public key: recovered={recovered:?} provided={provided:?}")]
    PublicKeyRecoveryMismatch {
        recovered: Box<k256::ecdsa::VerifyingKey>,
        provided: Box<k256::ecdsa::VerifyingKey>,
    },
    #[error("Invalid ECDSA public key bytes: {0}")]
    InvalidEcdsaPublicKeyBytes(#[source] k256::ecdsa::Error),
    #[error("Invalid Ed25519 public key bytes: {0}")]
    InvalidEd25519PublicKeyBytes(#[source] ed25519_dalek::SignatureError),
    #[error("Ed2559 Signature verification failed : {0}")]
    Ed25519VerificationFailed(#[source] ed25519_dalek::SignatureError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{random_bytes, random_valid_weighted_signature};

    #[test]
    fn valid_weighted_signature() {
        let message = random_bytes::<32>();
        let weighted_signature = random_valid_weighted_signature(&message);
        let serialized = rkyv::to_bytes::<_, 1024>(&weighted_signature).unwrap();
        let weighted_signature = unsafe { rkyv::archived_root::<WeightedSignature>(&serialized) };

        assert!(weighted_signature.verify(&message).is_ok())
    }

    #[test]
    fn invalid_weighted_signature() {
        use SignatureVerificationError::*;

        let message = random_bytes::<32>();
        let weighted_signature = random_valid_weighted_signature(&message);
        let serialized = rkyv::to_bytes::<_, 1024>(&weighted_signature).unwrap();
        let weighted_signature = unsafe { rkyv::archived_root::<WeightedSignature>(&serialized) };

        // Use another message for verification
        let other_message = random_bytes::<32>();

        assert!(matches!(
            weighted_signature.verify(&other_message).unwrap_err(),
            EcdsaVerificationFailed(_)
                | Ed25519VerificationFailed(_)
                | PublicKeyRecoveryMismatch { .. }
        ));
    }
}
