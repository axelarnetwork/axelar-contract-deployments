//! Address
// TODO: this whole module could be replaced by a trait on [u8; 33].
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

/// Represents an ECDSA public key.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug, PartialOrd, Copy, Eq)]
pub struct Address([u8; Self::ECDSA_COMPRESSED_PUBKEY_LEN]);

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
        hex::decode(value)?.as_slice().try_into()
    }
}

impl TryFrom<&[u8]> for Address {
    type Error = AddressError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != Self::ECDSA_COMPRESSED_PUBKEY_LEN {
            Err(AddressError::InvalidLength(bytes.len()))
        } else {
            // Unwrap: we just checked that the input size matches the inner array.
            Ok(Self(bytes.try_into().unwrap()))
        }
    }
}

impl From<[u8; Address::ECDSA_COMPRESSED_PUBKEY_LEN]> for Address {
    fn from(value: [u8; Address::ECDSA_COMPRESSED_PUBKEY_LEN]) -> Self {
        Self(value)
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
    use test_fixtures::primitives::bytes;

    use super::*;

    #[test]
    fn test_address_from_hex() -> Result<()> {
        let bytes = bytes(33);
        let hex = hex::encode(&bytes);
        let addr = Address::try_from(hex.as_str())?;

        assert_eq!(addr.0.as_slice(), &bytes);
        Ok(())
    }
}
