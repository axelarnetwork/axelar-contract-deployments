//! Signature

use std::array::TryFromSliceError;

use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHexError;

/// Error variants for [Signature].
#[derive(Debug)]
pub enum SignatureError {
    /// When couldn't decode given hex.
    FromHexDecode(FromHexError),
    /// When couldn't read returned vec as slice.
    FromHexAsSlice(TryFromSliceError),
    /// When given signature length isn't the expected.
    InvalidSignatureLen,
}

/// [Signature] represents ECDSA signature with apended 1-byte recovery id.
#[derive(BorshSerialize, BorshDeserialize, Clone, PartialEq, Debug)]
pub struct Signature(Vec<u8>);

impl<'a> Signature {
    const ECDSA_SIGNATURE_LEN: usize = 64;

    /// Constructor for [Signature].
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Returns last byte of the signature aka recovery id.
    pub fn recovery_id(&'a self) -> &'a u8 {
        &self.0[63]
    }

    /// Returns first 63 byte of the signature.
    pub fn signature(&'a self) -> &'a [u8] {
        &self.0
    }

    /// Signature from hex.
    pub fn from_hex(hex: &str) -> Result<Self, SignatureError> {
        let bytes = match hex::decode(hex) {
            Ok(v) => v,
            Err(e) => return Err(SignatureError::FromHexDecode(e)),
        };

        if bytes.len() != Self::ECDSA_SIGNATURE_LEN {
            return Err(SignatureError::InvalidSignatureLen);
        }

        Ok(Self(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_id() {
        let (bytes, _) = get_testdata_bytes_hex();
        let signature = Signature::new(bytes.to_vec());

        let actual = signature.recovery_id();
        let expected = 2;

        assert_eq!(&expected, actual)
    }

    #[test]
    fn test_signature_from_hex() {
        let (bytes, hex) = get_testdata_bytes_hex();
        let sig = Signature::from_hex(hex).unwrap();

        assert_eq!(sig.0, bytes)
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
