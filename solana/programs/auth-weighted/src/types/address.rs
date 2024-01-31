//! Address

use std::array::TryFromSliceError;

use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHexError;
use thiserror::Error;

/// Error variants for [AddressError].
#[derive(Error, Debug)]
pub enum AddressError {
    /// When couldn't decode given hex.
    #[error(transparent)]
    FromHexDecode(#[from] FromHexError),

    /// When couldn't read returned vec as slice.
    #[error(transparent)]
    FromHexAsSlice(#[from] TryFromSliceError),

    /// When given [Address] length isn't the expected.
    #[error("Invalid address length: {0}")]
    InvalidLength(usize),
}

/// [Address] represents ECDSA public key.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug, PartialOrd)]
pub struct Address(Vec<u8>);

impl AsRef<[u8]> for Address {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8]> for Address {
    fn eq(&self, other: &[u8]) -> bool {
        self.0 == other
    }
}

impl TryFrom<&str> for Address {
    type Error = AddressError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        hex::decode(value)?.try_into()
    }
}

impl TryFrom<Vec<u8>> for Address {
    type Error = AddressError;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() != Self::ECDSA_COMPRESSED_PUBKEY_LEN {
            Err(AddressError::InvalidLength(bytes.len()))
        } else {
            Ok(Self(bytes))
        }
    }
}

impl Address {
    /// Size of the ECDSA compressed public key in bytes.
    pub const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

    /// Returns [ECDSA_COMPRESSED_PUBKEY_LEN] value.
    pub fn expected_len() -> usize {
        Self::ECDSA_COMPRESSED_PUBKEY_LEN
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
    use anyhow::Result;

    use super::*;
    #[test]
    fn test_address_from_hex() -> Result<()> {
        let bytes = [
            0x03, 0xf5, 0x7d, 0x1a, 0x81, 0x3f, 0xeb, 0xac, 0xcb, 0xe6, 0x42, 0x96, 0x03, 0xf9,
            0xec, 0x57, 0x96, 0x95, 0x11, 0xb7, 0x6c, 0xd6, 0x80, 0x45, 0x2d, 0xba, 0x91, 0xfa,
            0x01, 0xf5, 0x4e, 0x75, 0x6d,
        ];
        let hex = "03f57d1a813febaccbe6429603f9ec57969511b76cd680452dba91fa01f54e756d";
        let addr = Address::try_from(hex)?;

        assert_eq!(addr.0, bytes);
        Ok(())
    }
}
