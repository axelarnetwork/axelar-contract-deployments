use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};

/// Custom u128 type with 8-byte alignment instead of the default 16-byte alignment.
///
/// This type is required for zero-copy accounts. The standard `u128` type
/// has 16-byte alignment, which creates a misalignment issue with Anchor style
/// 8-byte discriminator:
/// - Account discriminator occupies bytes 0-7 (8 bytes)
/// - Account data starts at byte 8
/// - With 16-byte alignment, the data at byte 8 is not properly aligned for `u128`
/// - This causes `bytemuck::from_bytes` to fail during deserialization
///
/// By using `[u8; 16]` as the underlying representation, we achieve 8-byte alignment
/// while maintaining the same byte layout as `u128` (little-endian).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Pod, Zeroable)]
#[repr(C)]
pub struct U128([u8; 16]);

impl U128 {
    pub const ZERO: Self = Self([0u8; 16]);
    pub const MAX: Self = Self([0xFF; 16]);

    #[allow(clippy::little_endian_bytes)]
    pub const fn new(value: u128) -> Self {
        Self(value.to_le_bytes())
    }

    #[allow(clippy::little_endian_bytes)]
    pub const fn get(self) -> u128 {
        u128::from_le_bytes(self.0)
    }

    #[must_use]
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.get().checked_add(other.get()).map(Self::new)
    }

    #[must_use]
    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(self.get().saturating_add(other.get()))
    }

    #[must_use]
    pub fn saturating_add_u128(self, other: u128) -> Self {
        Self::new(self.get().saturating_add(other))
    }

    #[must_use]
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.get().checked_sub(other.get()).map(Self::new)
    }

    #[must_use]
    pub fn saturating_sub(self, other: Self) -> Self {
        Self::new(self.get().saturating_sub(other.get()))
    }
}

impl From<u128> for U128 {
    fn from(value: u128) -> Self {
        Self::new(value)
    }
}

impl From<U128> for u128 {
    fn from(value: U128) -> Self {
        value.get()
    }
}

impl From<u64> for U128 {
    fn from(value: u64) -> Self {
        Self::new(value as u128)
    }
}

// Implement BorshSerialize/Deserialize to serialize as u128 in IDL
impl BorshSerialize for U128 {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.0)
    }
}

impl BorshDeserialize for U128 {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut bytes = [0u8; 16];
        reader.read_exact(&mut bytes)?;
        Ok(Self(bytes))
    }
}

// Display implementation
impl std::fmt::Display for U128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get())
    }
}

#[cfg(test)]
#[allow(clippy::little_endian_bytes)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_layout_compatibility() {
        // Verify U128 has identical byte representation to u128
        let test_value = 0x0102_0304_0506_0708_090a_0b0c_0d0e_0f10_u128;

        let u128_bytes = test_value.to_le_bytes();
        let custom_u128 = U128::new(test_value);

        assert_eq!(custom_u128.0, u128_bytes);
        assert_eq!(custom_u128.get(), test_value);
    }

    #[test]
    fn test_constants() {
        assert_eq!(U128::ZERO.get(), 0);
        assert_eq!(U128::MAX.get(), u128::MAX);
    }

    #[test]
    fn test_arithmetic() {
        let a = U128::new(100);
        let b = U128::new(50);

        assert_eq!(a.saturating_add(b).get(), 150);
        assert_eq!(a.saturating_sub(b).get(), 50);
        assert_eq!(a.checked_add(b).unwrap().get(), 150);
        assert_eq!(a.checked_sub(b).unwrap().get(), 50);

        // Test overflow
        assert_eq!(U128::MAX.saturating_add(U128::new(1)), U128::MAX);
        assert!(U128::MAX.checked_add(U128::new(1)).is_none());

        // Test underflow
        assert_eq!(U128::ZERO.saturating_sub(U128::new(1)), U128::ZERO);
        assert!(U128::ZERO.checked_sub(U128::new(1)).is_none());
    }

    #[test]
    fn test_conversions() {
        #[allow(clippy::unreadable_literal)]
        let value = 0x123456789abcdef0_u128;

        // From u128
        let custom: U128 = value.into();
        assert_eq!(custom.get(), value);

        // To u128
        let back: u128 = custom.into();
        assert_eq!(back, value);

        // From u64
        let from_u64: U128 = 42u64.into();
        assert_eq!(from_u64.get(), 42u128);
    }
}
