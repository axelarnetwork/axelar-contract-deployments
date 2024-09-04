use bnum::cast::As;
use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, Copy, Archive, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
#[archive(compare(PartialEq, PartialOrd))]
#[archive_attr(derive(Debug, PartialEq, Eq, Ord, PartialOrd, CheckBytes))]
pub struct U128([u8; 16]);

impl std::fmt::Debug for U128 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", u128::from(*self))
    }
}

impl U128 {
    pub const ZERO: U128 = U128([0u8; 16]);

    pub fn from_le(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    pub fn to_le(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let a: bnum::types::U128 = self.into();
        a.checked_add(other.into()).map(|res| res.into())
    }
}

impl ArchivedU128 {
    pub(crate) fn to_le(&self) -> &[u8; 16] {
        &self.0
    }

    pub(crate) fn into_le(self) -> [u8; 16] {
        self.0
    }
}

impl From<u128> for U128 {
    fn from(value: u128) -> Self {
        Self(value.to_le_bytes())
    }
}

impl From<U128> for u128 {
    fn from(value: U128) -> Self {
        u128::from_le_bytes(*value.to_le())
    }
}

impl From<ArchivedU128> for u128 {
    fn from(value: ArchivedU128) -> Self {
        u128::from_le_bytes(value.into_le())
    }
}

impl From<&ArchivedU128> for u128 {
    fn from(value: &ArchivedU128) -> Self {
        u128::from_le_bytes(*value.to_le())
    }
}

impl From<u128> for ArchivedU128 {
    fn from(value: u128) -> Self {
        Self(value.to_le_bytes())
    }
}

impl From<bnum::types::U128> for U128 {
    fn from(value: bnum::types::U128) -> Self {
        // Using Bnums primitive type trait as conversion as proxy.
        // https://docs.rs/bnum/latest/bnum/types/type.U128.html#impl-AsPrimitive%3Cu128%3E-for-BUint%3CN%3E
        let primitive: u128 = value.as_();
        U128::from_le(primitive.to_le_bytes())
    }
}

impl From<U128> for bnum::types::U128 {
    fn from(val: U128) -> Self {
        // Unwrap: Our U128 type has the expected number of bytes, so this never panics.
        bnum::types::U128::from_le_slice(val.to_le()).unwrap()
    }
}

impl From<&ArchivedU128> for bnum::types::U128 {
    fn from(val: &ArchivedU128) -> Self {
        // Unwrap: Our ArchivedU128 type has the expected number of bytes, so this never
        // panics.
        bnum::types::U128::from_le_slice(val.to_le()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use bnum::types::U128 as BnumU128;

    use super::*;
    use crate::test_fixtures::random_bytes;

    #[test]
    fn test_endianness() {
        let bytes = random_bytes::<16>();
        let u128 = U128::from_le(bytes);
        assert_eq!(*u128.to_le(), bytes);
    }

    #[test]
    fn bnum_round_trip() {
        let bytes = random_bytes::<16>();

        let u128 = U128::from_le(bytes);
        let bnum = BnumU128::from_le_slice(&bytes).unwrap();

        let bnum_converted: BnumU128 = u128.into();
        let u128_converted: U128 = bnum.into();

        assert_eq!(u128, u128_converted);
        assert_eq!(bnum, bnum_converted);
    }

    #[test]
    fn test_u128_from_u128() {
        const SIZE: usize = u128::BITS as usize >> 3;

        // Min
        let min = U128::from(u128::MIN);
        assert_eq!(min, U128::ZERO);

        // Max
        let max: u128 = u128::MAX;
        let expected_max = {
            let mut buffer = [0u8; 16];
            buffer[..SIZE].copy_from_slice(&max.to_le_bytes());
            buffer
        };
        assert_eq!(U128::from(max).0, expected_max);

        // Mid
        let mid = max >> 1;
        let expected_intermediate = {
            let mut buffer = [0u8; 16];
            buffer[..SIZE].copy_from_slice(&mid.to_le_bytes());
            buffer
        };
        assert_eq!(U128::from(mid).0, expected_intermediate);
    }
}
