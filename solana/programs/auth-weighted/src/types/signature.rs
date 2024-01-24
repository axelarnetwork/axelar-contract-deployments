//! Signature

use borsh::{BorshDeserialize, BorshSerialize};
use libsecp256k1::RecoveryId;
use solana_program::secp256k1_recover::Secp256k1Pubkey;
use thiserror::Error;

use crate::error::AuthWeightedError;

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
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub struct Signature {
    signature: [u8; Self::ECDSA_SIGNATURE_LEN],
    recovery_id: u8,
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

    /// Recovers the public key on Solana runtime
    #[inline]
    pub fn sol_recover_public_key(
        &self,
        message_hash: &[u8],
    ) -> Result<Secp256k1Pubkey, AuthWeightedError> {
        if message_hash.len() != 32 {
            return Err(AuthWeightedError::Secp256k1RecoveryFailedInvalidHash);
        }

        Ok(solana_program::secp256k1_recover::secp256k1_recover(
            message_hash,
            self.recovery_id,
            &self.signature,
        )?)
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

impl From<SignatureError> for AuthWeightedError {
    fn from(signature_error: SignatureError) -> Self {
        use AuthWeightedError::*;
        use SignatureError::*;
        match signature_error {
            InvalidLength => Secp256k1InvalidSignature,
            InvalidRecoveryId => Secp256k1RecoveryFailedInvalidRecoveryId,
            InvalidSignatureBytes => Secp256k1RecoveryFailedInvalidSignature,
            PubKeyRecoveryFailed => Secp256k1RecoveryFailed,
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use test_fixtures::ecdsa_signature::{create_random_signature, TestSignature};
    use test_fixtures::random_stuff::bytes;

    use super::*;

    #[test]
    fn test_recovery_id() -> Result<()> {
        let TestSignature {
            signature,
            recovery_id,
            ..
        } = create_random_signature(&bytes(100));

        let mut input = signature.serialize().to_vec();
        input.push(recovery_id.serialize());

        let ours = Signature::try_from(input)?;

        assert_eq!(*ours.signature_bytes(), signature.serialize());
        assert_eq!(ours.recovery_id_byte(), recovery_id.serialize());

        Ok(())
    }
}
