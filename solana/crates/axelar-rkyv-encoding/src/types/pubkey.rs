use rkyv::{Archive, Deserialize, Serialize};

pub const ED25519_PUBKEY_LEN: usize = 32;
pub const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

pub type EcdsaPubkey = [u8; ECDSA_COMPRESSED_PUBKEY_LEN];
pub type Ed25519Pubkey = [u8; ED25519_PUBKEY_LEN];

#[derive(Archive, Deserialize, Serialize, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Debug)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub enum PublicKey {
    Ecdsa(EcdsaPubkey),
    Ed25519(Ed25519Pubkey),
}

impl PublicKey {
    pub fn new_ecdsa(pubkey: EcdsaPubkey) -> Self {
        PublicKey::Ecdsa(pubkey)
    }

    pub fn new_ed25519(pubkey: Ed25519Pubkey) -> Self {
        PublicKey::Ed25519(pubkey)
    }
}
