use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

pub const ED25519_SIGNATURE_LEN: usize = 64;
pub const ECDSA_RECOVERABLE_SIGNATURE_LEN: usize = 65;

pub type EcdsaRecoverableSignature = [u8; ECDSA_RECOVERABLE_SIGNATURE_LEN];
pub type Ed25519Signature = [u8; ED25519_SIGNATURE_LEN];

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum Signature {
    EcdsaRecoverable(EcdsaRecoverableSignature),
    Ed25519(Ed25519Signature),
}

impl Signature {
    pub fn new_ecdsa_recoverable(bytes: EcdsaRecoverableSignature) -> Self {
        Self::EcdsaRecoverable(bytes)
    }

    pub fn new_ed25519(bytes: Ed25519Signature) -> Self {
        Self::Ed25519(bytes)
    }
}
