use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use crate::types::{
    EcdsaRecoverableSignature, Ed25519Pubkey, Ed25519Signature, Secp256k1Pubkey, Signature, U256,
};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct WeightedSigner {
    pub signature: Option<Signature>,
    pub weight: U256,
}

impl WeightedSigner {
    pub fn new(signature: Option<Signature>, weight: U256) -> Self {
        Self { signature, weight }
    }
}

impl ArchivedWeightedSigner {
    pub(crate) fn verify_ecdsa(
        signature_bytes: &EcdsaRecoverableSignature,
        public_key_bytes: &Secp256k1Pubkey,
        message: &[u8; 32],
    ) -> Result<(), SignatureVerificationError> {
        use libsecp256k1::{recover, Message, RecoveryId, Signature};
        let message = Message::parse(message);
        let (signature_bytes, recovery_id) = match signature_bytes {
            [first_64 @ .., recovery_id] => (first_64, recovery_id),
        };
        let signature = Signature::parse_standard(signature_bytes)
            .map_err(SignatureVerificationError::Libsecp256k1Error)?;
        let recovery_id = RecoveryId::parse(*recovery_id)
            .map_err(SignatureVerificationError::Libsecp256k1Error)?;
        let recovered_key = recover(&message, &signature, &recovery_id)
            .map_err(SignatureVerificationError::Libsecp256k1Error)?;
        let recovered_key = recovered_key.serialize_compressed();
        if &recovered_key == public_key_bytes {
            return Ok(());
        }
        Err(SignatureVerificationError::EcdsaVerificationFailed)
    }

    pub(crate) fn verify_ed25519(
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
    #[error("Failed to recover ECDSA public key ")]
    PublicKeyRecovery,
    #[error("ECDSA Signature verification failed")]
    EcdsaVerificationFailed,
    /// We cannot use `libsecp256k1` with the `std` feature because it pulls in
    /// `rand` which does not work in Solana bpf. This means that it does not
    /// implement `Error` trait and we cannot use `#[from]`
    #[error("libsecpk256k1 error: {0}")]
    Libsecp256k1Error(libsecp256k1::Error),
    /// Fields are boxed to reduce the amount of stack space reserved.
    #[error("Recovered public key is different than provided public key: recovered={recovered:?} provided={provided:?}")]
    PublicKeyRecoveryMismatch {
        recovered: Box<libsecp256k1::PublicKey>,
        provided: Box<libsecp256k1::PublicKey>,
    },
    #[error("Invalid ECDSA public key bytes")]
    InvalidEcdsaPublicKeyBytes,
    #[error("Invalid Ed25519 public key bytes: {0}")]
    InvalidEd25519PublicKeyBytes(#[source] ed25519_dalek::SignatureError),
    #[error("Ed2559 Signature verification failed : {0}")]
    Ed25519VerificationFailed(#[source] ed25519_dalek::SignatureError),
}
