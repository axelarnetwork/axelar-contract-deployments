//! U256 implementation of uint256.

use borsh::{BorshDeserialize, BorshSerialize};

/// [U256] represents uint256.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Ord, Eq, Hash)]
pub struct U256(ethnum::U256);

impl U256 {
    /// The additive identity for this integer type, i.e. `0`.
    pub const ZERO: U256 = Self(ethnum::U256::ZERO);
    /// The multiplicative identity for this integer type, i.e. `1`.
    pub const ONE: U256 = Self(ethnum::U256::ONE);

    /// Create an integer value from its representation as a byte array in
    /// little endian.
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        U256(ethnum::U256::from_le_bytes(bytes))
    }

    /// Checked integer addition. Computes `self + rhs`, returning `None` if
    /// overflow occurred.
    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Checked integer subtraction. Computes `self - rhs`, returning `None` if
    /// overflow occurred.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl From<u8> for U256 {
    fn from(value: u8) -> Self {
        U256(ethnum::U256::from(value))
    }
}

impl BorshSerialize for U256 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        // It doesn't return error at all.
        self.0 .0.serialize(writer).unwrap();
        Ok(())
    }
}

impl BorshDeserialize for U256 {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let mut buffer = [0u8; 32];
        reader.read_exact(&mut buffer)?;
        let inner = ethnum::u256::from_le_bytes(buffer);
        Ok(U256(inner))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_u256_roundtrip() {
        let expected = U256::from_le_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ]);

        let not_expected = U256::from_le_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x22,
        ]);

        let mut buffer: Vec<u8> = Vec::new();
        expected.serialize(&mut buffer).unwrap();

        let mut cursor = Cursor::new(&buffer);
        let actual = U256::deserialize_reader(&mut cursor).unwrap();

        assert_eq!(expected, actual);
        assert_ne!(not_expected, actual)
    }
}
