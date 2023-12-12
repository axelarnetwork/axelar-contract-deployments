//! Address

use std::array::TryFromSliceError;

use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHexError;

/// Error variants for [AddressError].
#[derive(Debug)]
pub enum AddressError {
    /// When couldn't decode given hex.
    FromHexDecode(FromHexError),
    /// When couldn't read returned vec as slice.
    FromHexAsSlice(TryFromSliceError),
    /// When given [Address] length isn't the expected.
    InvalidAddressLen,
}

/// [Address] represents ECDSA public key.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub struct Address(Vec<u8>);

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8; 64]> for Address {
    fn eq(&self, other: &[u8; 64]) -> bool {
        self.0.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl PartialEq<[u8; 32]> for Address {
    fn eq(&self, other: &[u8; 32]) -> bool {
        self.0.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl PartialEq<[u8]> for Address {
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == other
    }
}

impl Address {
    const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

    /// Returns [ECDSA_COMPRESSED_PUBKEY_LEN] value.
    pub fn expected_len() -> usize {
        Self::ECDSA_COMPRESSED_PUBKEY_LEN
    }

    /// Constructor for [Address].
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Signature from hex.
    pub fn from_hex(hex: &str) -> Result<Self, AddressError> {
        let bytes = match hex::decode(hex) {
            Ok(v) => v,
            Err(e) => return Err(AddressError::FromHexDecode(e)),
        };

        if bytes.len() != Self::ECDSA_COMPRESSED_PUBKEY_LEN {
            return Err(AddressError::InvalidAddressLen);
        }

        Ok(Self::new(bytes))
    }

    /// Returns ECDSA public key (compressed) without prefix.
    pub fn omit_prefix(&self) -> [u8; 32] {
        let mut result = [0; 32];
        result.copy_from_slice(&self.0[1..]);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_from_hex() {
        let bytes = [
            0x03, 0xf5, 0x7d, 0x1a, 0x81, 0x3f, 0xeb, 0xac, 0xcb, 0xe6, 0x42, 0x96, 0x03, 0xf9,
            0xec, 0x57, 0x96, 0x95, 0x11, 0xb7, 0x6c, 0xd6, 0x80, 0x45, 0x2d, 0xba, 0x91, 0xfa,
            0x01, 0xf5, 0x4e, 0x75, 0x6d,
        ];
        let hex = "03f57d1a813febaccbe6429603f9ec57969511b76cd680452dba91fa01f54e756d";
        let addr = Address::from_hex(hex).unwrap();

        assert_eq!(addr.0, bytes)
    }
}
