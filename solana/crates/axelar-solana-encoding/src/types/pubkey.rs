//! # Cryptographic Primitives Module
//!
//! This module defines essential cryptographic types and constants used for
//! handling public keys and signatures within the system. It supports multiple
//! cryptographic algorithms, including Secp256k1 and Ed25519, providing a
//! unified interface for public key and signature management.

/// The length of an Ed25519 public key in bytes.
pub const ED25519_PUBKEY_LEN: usize = 32;

/// The length of a compressed Secp256k1 public key in bytes.
pub const SECP256K1_COMPRESSED_PUBKEY_LEN: usize = 33;

/// Type alias for a compressed Secp256k1 public key.
pub type Secp256k1Pubkey = [u8; SECP256K1_COMPRESSED_PUBKEY_LEN];

/// Type alias for an Ed25519 public key.
pub type Ed25519Pubkey = [u8; ED25519_PUBKEY_LEN];

/// Represents a public key using supported cryptographic algorithms.
#[derive(
    Clone,
    Copy,
    Ord,
    PartialOrd,
    PartialEq,
    Eq,
    udigest::Digestable,
    borsh::BorshDeserialize,
    borsh::BorshSerialize,
)]
pub enum PublicKey {
    /// Compressed Secp256k1 public key.
    Secp256k1(Secp256k1Pubkey),

    /// Ed25519 public key.
    Ed25519(Ed25519Pubkey),
}

#[allow(clippy::min_ident_chars)]
impl core::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Secp256k1(pubkey) => {
                let hex = hex::encode(pubkey);
                f.write_str(hex.as_str())
            }
            Self::Ed25519(pubkey) => {
                let base58 = bs58::encode(pubkey).into_string();
                f.write_str(base58.as_str())
            }
        }
    }
}

/// The length of an Ed25519 signature in bytes.
pub const ED25519_SIGNATURE_LEN: usize = 64;

/// The length of a recoverable ECDSA signature in bytes.
pub const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = 65;

/// Type alias for a recoverable ECDSA signature.
pub type EcdsaRecoverableSignature = [u8; ECDSA_RECOVERABLE_SIGNATURE_LEN];

/// Type alias for an Ed25519 signature.
pub type Ed25519Signature = [u8; ED25519_SIGNATURE_LEN];

/// Represents a digital signature using supported cryptographic algorithms.
#[derive(Eq, PartialEq, Clone, Copy, borsh::BorshDeserialize, borsh::BorshSerialize)]
pub enum Signature {
    /// Recoverable ECDSA signature.
    EcdsaRecoverable(EcdsaRecoverableSignature),

    /// Ed25519 signature.
    Ed25519(Ed25519Signature),
}

#[allow(clippy::min_ident_chars)]
impl core::fmt::Debug for Signature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EcdsaRecoverable(sig) => {
                write!(f, "EcdsaRecoverable({})", hex::encode(sig))
            }
            Self::Ed25519(sig) => {
                write!(f, "Ed25519({})", hex::encode(sig))
            }
        }
    }
}
