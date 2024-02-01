//! U256 implementation of uint256.

use std::fmt::Display;

use borsh::{BorshDeserialize, BorshSerialize};

/// [U256] represents uint256.
#[derive(Copy, Clone, PartialEq, PartialOrd, Debug, Ord, Eq, Hash)]
pub struct U256(bnum::types::U256);

impl U256 {
    /// The additive identity for this integer type, i.e. `0`.
    pub const ZERO: U256 = Self(bnum::types::U256::ZERO);
    /// The multiplicative identity for this integer type, i.e. `1`.
    pub const ONE: U256 = Self(bnum::types::U256::ONE);
    /// Create an integer value from its representation as a byte array in
    /// little endian.
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        let cast: [u64; 4] = bytemuck::cast(bytes);
        U256(bnum::types::U256::from(cast))
    }

    /// Return the memory representation of this integer as a byte array in
    /// little-endian byte order.
    pub fn to_le_bytes(self) -> [u8; 32] {
        let bytes: [u64; 4] = self.0.into();
        bytemuck::cast(bytes)
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
        U256(bnum::types::U256::from(value))
    }
}

impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        U256(bnum::types::U256::from(value))
    }
}

impl BorshSerialize for U256 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> Result<(), std::io::Error> {
        let bytes = self.to_le_bytes();
        bytes.serialize(writer)
    }
}

impl BorshDeserialize for U256 {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> Result<Self, std::io::Error> {
        let mut buffer = [0u8; 32];
        reader.read_exact(&mut buffer)?;
        Ok(U256::from_le_bytes(buffer))
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string())
    }
}
