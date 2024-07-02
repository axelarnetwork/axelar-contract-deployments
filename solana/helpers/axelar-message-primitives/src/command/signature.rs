//! Signature

use borsh::{BorshDeserialize, BorshSerialize};
use libsecp256k1::RecoveryId;
use thiserror::Error;

/// Error variants for [SignatureError].
#[derive(Error, Debug)]
pub enum SignatureError {
    /// When given [Signature] length isn't the expected.
    #[error("Invalid signature length")]
    InvalidLength,

    /// Invalid recovery id
    #[error("Invalid recovery id")]
    InvalidRecoveryId,

    /// Invalid signature
    #[error("Invalid signature")]
    InvalidSignatureBytes,

    /// Public key recovery failed
    #[error("Public key recovery failed")]
    PubKeyRecoveryFailed,
}

/// Wrapper type to hold bytes and handle serialization for the signed bytes and
/// its recovery id of an ECDSA signature..
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Signature {
    pub signature: [u8; Self::ECDSA_SIGNATURE_LEN],
    pub recovery_id: u8,
}

impl Signature {
    /// Signature size in bytes.
    pub const ECDSA_SIGNATURE_LEN: usize = 64;

    /// Signature and recovery id size in bytes.
    pub const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = Self::ECDSA_SIGNATURE_LEN + 1;

    fn new(
        signature: [u8; Self::ECDSA_SIGNATURE_LEN],
        recovery_id: u8,
    ) -> Result<Self, SignatureError> {
        if RecoveryId::parse(recovery_id).is_ok() {
            Ok(Self {
                signature,
                recovery_id,
            })
        } else {
            Err(SignatureError::InvalidRecoveryId)
        }
    }

    /// The recovery id as a byte
    #[inline]
    pub fn recovery_id_byte(&self) -> u8 {
        self.recovery_id
    }

    /// The signature bytes.
    #[inline]
    pub fn signature_bytes(&self) -> &[u8; Self::ECDSA_SIGNATURE_LEN] {
        &self.signature
    }
}

impl TryFrom<&str> for Signature {
    type Error = SignatureError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        hex::decode(value)
            .map_err(|_| SignatureError::InvalidLength)?
            .try_into()
    }
}

impl TryFrom<Vec<u8>> for Signature {
    type Error = SignatureError;

    fn try_from(mut bytes: Vec<u8>) -> Result<Self, Self::Error> {
        match bytes.len() {
            Self::ECDSA_RECOVERABLE_SIGNATURE_LEN => {
                // Pop out the recovery byte.
                // Unwrap: we just checked it have 65 elements.
                let recovery_id = bytes.pop().unwrap();
                let signature: [u8; Self::ECDSA_SIGNATURE_LEN] = bytes.try_into().unwrap();
                Self::new(signature, recovery_id)
            }
            _ => Err(SignatureError::InvalidLength),
        }
    }
}
