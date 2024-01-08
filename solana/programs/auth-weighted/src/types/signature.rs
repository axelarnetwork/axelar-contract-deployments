//! Signature

use std::array::TryFromSliceError;

use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHexError;
use thiserror::Error;

/// Error variants for [SignatureError].
#[derive(Error, Debug)]
pub enum SignatureError {
    /// When couldn't decode given hex.
    #[error(transparent)]
    FromHexDecode(#[from] FromHexError),

    /// When couldn't read returned vec as slice.
    #[error(transparent)]
    FromHexAsSlice(#[from] TryFromSliceError),

    /// When given [Signature] length isn't the expected.
    #[error("Invalid signature length: {0}")]
    InvalidLength(usize),
}

/// [Signature] represents ECDSA signature with apended 1-byte recovery id.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub struct Signature(Vec<u8>);

impl<'a> Signature {
    /// Signature size in bytes.
    pub const ECDSA_SIGNATURE_LEN: usize = 64;

    /// Returns last byte of the signature aka recovery id.
    pub fn recovery_id(&'a self) -> &'a u8 {
        &self.0[63]
    }

    /// Returns first 63 byte of the signature.
    pub fn signature(&'a self) -> &'a [u8] {
        &self.0
    }
}

impl TryFrom<&str> for Signature {
    type Error = SignatureError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        hex::decode(value)?.try_into()
    }
}

impl TryFrom<Vec<u8>> for Signature {
    type Error = SignatureError;

    fn try_from(mut bytes: Vec<u8>) -> Result<Self, Self::Error> {
        match bytes.len() {
            64 => Ok(Self(bytes)),
            65 => {
                // Pop out the recovery byte.
                // Unwrap: we just checked it have 65 elements.
                bytes.pop().unwrap();
                Ok(Self(bytes))
            }
            _ => Err(SignatureError::InvalidLength(bytes.len())),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_recovery_id() -> Result<()> {
        let (bytes, _) = get_testdata_bytes_hex();
        let signature = Signature::try_from(bytes.to_vec())?;

        let actual = signature.recovery_id();
        let expected = 2;

        assert_eq!(&expected, actual);
        Ok(())
    }

    #[test]
    fn test_signature_from_hex() -> Result<()> {
        let (bytes, hex) = get_testdata_bytes_hex();
        let sig = Signature::try_from(hex)?;

        assert_eq!(sig.0, bytes);
        Ok(())
    }

    fn get_testdata_bytes_hex() -> ([u8; 64], &'static str) {
        let bytes = [
            0x28, 0x37, 0x86, 0xd8, 0x44, 0xa7, 0xc4, 0xd1, 0xd4, 0x24, 0x83, 0x70, 0x74, 0xd0,
            0xc8, 0xec, 0x71, 0xbe, 0xcd, 0xcb, 0xa4, 0xdd, 0x42, 0xb5, 0x30, 0x7c, 0xb5, 0x43,
            0xa0, 0xe2, 0xc8, 0xb8, 0x1c, 0x10, 0xad, 0x54, 0x1d, 0xef, 0xd5, 0xce, 0x84, 0xd2,
            0xa6, 0x08, 0xfc, 0x45, 0x48, 0x27, 0xd0, 0xb6, 0x5b, 0x48, 0x65, 0xc8, 0x19, 0x2a,
            0x2e, 0xa1, 0x73, 0x6a, 0x5c, 0x4b, 0x72, 0x02,
        ];
        let hex = "283786d844a7c4d1d424837074d0c8ec71becdcba4dd42b5307cb543a0e2c8b81c10ad541defd5ce84d2a608fc454827d0b65b4865c8192a2ea1736a5c4b7202";
        (bytes, hex)
    }
}
