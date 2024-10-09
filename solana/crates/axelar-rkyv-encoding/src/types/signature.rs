use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

pub const ED25519_SIGNATURE_LEN: usize = 64;
pub const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = 65;

pub type EcdsaRecoverableSignature = [u8; ECDSA_RECOVERABLE_SIGNATURE_LEN];
pub type Ed25519Signature = [u8; ED25519_SIGNATURE_LEN];

#[derive(Archive, Deserialize, Serialize, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Signature {
    EcdsaRecoverable(EcdsaRecoverableSignature),
    Ed25519(Ed25519Signature),
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Signature::EcdsaRecoverable(sig) => {
                write!(f, "EcdsaRecoverable({})", hex::encode(sig))
            }
            Signature::Ed25519(sig) => {
                write!(f, "Ed25519({})", hex::encode(sig))
            }
        }
    }
}

impl Signature {
    pub fn new_ecdsa_recoverable(bytes: EcdsaRecoverableSignature) -> Self {
        Self::EcdsaRecoverable(bytes)
    }

    pub fn new_ed25519(bytes: Ed25519Signature) -> Self {
        Self::Ed25519(bytes)
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        match self {
            Signature::EcdsaRecoverable(bytes) => bytes,
            Signature::Ed25519(bytes) => bytes,
        }
    }
}

impl AsRef<[u8]> for ArchivedSignature {
    fn as_ref(&self) -> &[u8] {
        match self {
            ArchivedSignature::EcdsaRecoverable(bytes) => bytes,
            ArchivedSignature::Ed25519(bytes) => bytes,
        }
    }
}

impl AsMut<[u8]> for Signature {
    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            Signature::EcdsaRecoverable(bytes) => bytes,
            Signature::Ed25519(bytes) => bytes,
        }
    }
}
