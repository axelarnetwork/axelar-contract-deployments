use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, Copy, Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Ord, PartialOrd)]
#[archive(compare(PartialEq, PartialOrd))]
#[archive_attr(derive(Debug, PartialEq, Eq, Ord, PartialOrd, CheckBytes))]
pub struct U256([u8; 32]);

impl U256 {
    pub const ZERO: U256 = U256([0u8; 32]);

    pub fn from_le(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_le(self) -> [u8; 32] {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        let a: bnum::types::U256 = self.into();
        a.checked_add(other.into()).map(|res| res.into())
    }

    pub fn maybe_u128(&self) -> Option<u128> {
        let num: bnum::types::U256 = (*self).into();
        num.try_into().ok()
    }
}
impl ArchivedU256 {
    pub(crate) fn to_le(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn maybe_u128(&self) -> Option<u128> {
        let num: bnum::types::U256 = self.into();
        num.try_into().ok()
    }
}

impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        const SIZE: usize = u128::BITS as usize >> 3;
        let mut result = [0u8; 32];
        result[..SIZE].copy_from_slice(&value.to_le_bytes());
        Self(result)
    }
}

impl From<bnum::types::U256> for U256 {
    fn from(value: bnum::types::U256) -> Self {
        let mut bytes = value.to_radix_le(256);
        bytes.resize(32, 0);
        // Unwrap: the `bytes` vector always has 32 elements.
        U256::from_le(bytes.try_into().unwrap())
    }
}

impl From<U256> for bnum::types::U256 {
    fn from(val: U256) -> Self {
        // Unwrap: Our U256 type has the expected number of bytes, so this never panics.
        bnum::types::U256::from_le_slice(&val.to_le()).unwrap()
    }
}

impl From<&ArchivedU256> for bnum::types::U256 {
    fn from(val: &ArchivedU256) -> Self {
        // Unwrap: Our ArchivedU256 type has the expected number of bytes, so this never
        // panics.
        bnum::types::U256::from_le_slice(val.to_le()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use bnum::types::U256 as BnumU256;

    use super::*;
    use crate::test_fixtures::random_bytes;

    #[test]
    fn test_endiannes() {
        let bytes = random_bytes::<32>();
        let u256 = U256::from_le(bytes);
        assert_eq!(u256.to_le(), bytes);
    }

    #[test]
    fn bnum_round_trip() {
        let bytes = random_bytes::<32>();

        let u256 = U256::from_le(bytes);
        let bnum = BnumU256::from_le_slice(&bytes).unwrap();

        let bnum_converted: BnumU256 = u256.into();
        let u256_converted: U256 = bnum.into();

        assert_eq!(u256, u256_converted);
        assert_eq!(bnum, bnum_converted);
    }

    #[test]
    fn test_u256_from_u128() {
        const SIZE: usize = u128::BITS as usize >> 3;

        // Min
        let min = U256::from(u128::MIN);
        assert_eq!(min, U256::ZERO);

        // Max
        let max: u128 = u128::MAX;
        let expected_max = {
            let mut buffer = [0u8; 32];
            buffer[..SIZE].copy_from_slice(&max.to_le_bytes());
            buffer
        };
        assert_eq!(U256::from(max).0, expected_max);

        // Mid
        let mid = max >> 1;
        let expected_intermediate = {
            let mut buffer = [0u8; 32];
            buffer[..SIZE].copy_from_slice(&mid.to_le_bytes());
            buffer
        };
        assert_eq!(U256::from(mid).0, expected_intermediate);
    }
}
