use std::str::FromStr;

use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

pub const ED25519_PUBKEY_LEN: usize = 32;
pub const SECP256K1_COMPRESSED_PUBKEY_LEN: usize = 33;

pub type Secp256k1Pubkey = [u8; SECP256K1_COMPRESSED_PUBKEY_LEN];
pub type Ed25519Pubkey = [u8; ED25519_PUBKEY_LEN];

#[derive(Archive, Deserialize, Serialize, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[archive(compare(PartialEq, PartialOrd))]
#[archive_attr(derive(PartialEq, Eq, Ord, PartialOrd, CheckBytes))]
pub enum PublicKey {
    Secp256k1(Secp256k1Pubkey),
    Ed25519(Ed25519Pubkey),
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicKey::Secp256k1(pubkey) => {
                let hex = hex::encode(pubkey);
                f.write_str(hex.as_str())
            }
            PublicKey::Ed25519(pubkey) => {
                let base58 = bs58::encode(pubkey).into_string();
                f.write_str(base58.as_str())
            }
        }
    }
}

impl std::fmt::Debug for ArchivedPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchivedPublicKey::Secp256k1(pubkey) => {
                let hex = hex::encode(pubkey);
                f.write_str(hex.as_str())
            }
            ArchivedPublicKey::Ed25519(pubkey) => {
                let base58 = bs58::encode(pubkey).into_string();
                f.write_str(base58.as_str())
            }
        }
    }
}

impl PublicKey {
    pub fn new_ecdsa(pubkey: Secp256k1Pubkey) -> Self {
        PublicKey::Secp256k1(pubkey)
    }

    pub fn new_ed25519(pubkey: Ed25519Pubkey) -> Self {
        PublicKey::Ed25519(pubkey)
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        match self {
            PublicKey::Secp256k1(bytes) => bytes,
            PublicKey::Ed25519(bytes) => bytes,
        }
    }
}

impl ArchivedPublicKey {
    pub fn to_bytes(&self) -> Vec<u8> {
        let bytes: &[u8] = match self {
            ArchivedPublicKey::Secp256k1(bytes) => bytes,
            ArchivedPublicKey::Ed25519(bytes) => bytes,
        };
        bytes.to_vec()
    }
}

impl FromStr for PublicKey {
    type Err = Box<dyn std::error::Error + Send + Sync>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let maybe_ecdsa = hex::decode(s).ok().as_deref().and_then(maybe_ecdsa_pubkey);

        let maybe_ed25519 = |s| {
            bs58::decode(s)
                .into_vec()
                .ok()
                .as_deref()
                .and_then(maybe_ed25519_pubkey)
        };

        maybe_ecdsa
            .or_else(|| maybe_ed25519(s))
            .ok_or("Failed to parse PublicKey".into())
    }
}

fn maybe_ed25519_pubkey(bytes: &[u8]) -> Option<PublicKey> {
    let bytes: Ed25519Pubkey = bytes.try_into().ok()?;
    // Verify if bytes represent a valid Ed25519 pubkey
    let _valid = ed25519_dalek::VerifyingKey::from_bytes(&bytes).ok()?;
    Some(PublicKey::Ed25519(bytes))
}

fn maybe_ecdsa_pubkey(bytes: &[u8]) -> Option<PublicKey> {
    // Verify if bytes represent a valid ECDSA pubkey
    let bytes: Secp256k1Pubkey = bytes.try_into().ok()?;
    let _valid = libsecp256k1::PublicKey::parse_compressed(&bytes).ok()?;
    Some(PublicKey::Secp256k1(bytes))
}

#[cfg(test)]
mod tests {

    use super::*;

    fn random_ecdsa_pubkey() -> libsecp256k1::PublicKey {
        use libsecp_rand::rngs::OsRng;
        let signing_key = libsecp256k1::SecretKey::random(&mut OsRng);
        libsecp256k1::PublicKey::from_secret_key(&signing_key)
    }

    fn random_ed25519_pubkey() -> ed25519_dalek::VerifyingKey {
        use rand::rngs::OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut OsRng);
        signing_key.verifying_key()
    }

    fn random_ecdsa_bytes() -> Secp256k1Pubkey {
        let bytes: Box<[u8; 33]> = Box::new(random_ecdsa_pubkey().serialize_compressed());
        *bytes
    }

    fn random_ed25519_bytes() -> Ed25519Pubkey {
        random_ed25519_pubkey().to_bytes()
    }

    #[test]
    fn test_maybe_ecdsa_pubkey() {
        let verifying_key_bytes = random_ecdsa_bytes();
        let expected = PublicKey::Secp256k1(verifying_key_bytes);
        let result = maybe_ecdsa_pubkey(verifying_key_bytes.as_ref());
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_maybe_ed25519_pubkey() {
        let verifying_key_bytes = random_ed25519_bytes();
        let expected = PublicKey::Ed25519(verifying_key_bytes);
        let result = maybe_ed25519_pubkey(&verifying_key_bytes);
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_maybe_ed25519_pubkey_bad_input() {
        // Bad scheme
        let bytes = random_ecdsa_bytes();
        assert!(maybe_ed25519_pubkey(&bytes).is_none())
    }

    #[test]
    fn test_maybe_ecdsa_pubkey_bad_input() {
        // Bad scheme
        let bytes = random_ed25519_bytes();
        assert!(maybe_ecdsa_pubkey(&bytes).is_none())
    }

    #[test]
    fn parse_from_ed25519_b58() {
        let pubkey_bytes = random_ed25519_bytes();
        let b58_string = bs58::encode(pubkey_bytes).into_string();
        let parsed: PublicKey = b58_string.parse().unwrap();
        let PublicKey::Ed25519(parsed_bytes) = parsed else {
            panic!()
        };
        assert_eq!(pubkey_bytes, parsed_bytes)
    }

    #[test]
    fn parse_from_ecdsa_hex() {
        let pubkey_bytes = random_ecdsa_bytes();
        let hex_string = hex::encode(pubkey_bytes);
        let parsed: PublicKey = hex_string.parse().unwrap();
        let PublicKey::Secp256k1(parsed_bytes) = parsed else {
            panic!()
        };
        assert_eq!(pubkey_bytes, parsed_bytes)
    }

    #[test]
    fn parse_invalid() {
        assert!(PublicKey::from_str("foo").is_err());
    }
}
