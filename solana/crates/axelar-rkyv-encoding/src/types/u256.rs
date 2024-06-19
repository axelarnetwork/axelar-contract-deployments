use rkyv::{Archive, Deserialize, Serialize};

#[derive(Clone, Copy, Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct U256([u8; 32]);

impl U256 {
    pub fn from_le(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn to_le(self) -> [u8; 32] {
        self.0
    }
}

impl ArchivedU256 {
    pub(crate) fn to_le(&self) -> &[u8; 32] {
        &self.0
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
    use rand::thread_rng;

    use super::*;
    use crate::tests::fixtures::random_bytes;

    #[test]
    fn test_endiannes() {
        let mut rng = thread_rng();
        let bytes = random_bytes::<32>(&mut rng);
        let u256 = U256::from_le(bytes);
        assert_eq!(u256.to_le(), bytes);
    }

    #[test]
    fn bnum_round_trip() {
        let mut rng = thread_rng();
        let bytes = random_bytes::<32>(&mut rng);

        let u256 = U256::from_le(bytes);
        let bnum = BnumU256::from_le_slice(&bytes).unwrap();

        let bnum_converted: BnumU256 = u256.into();
        let u256_converted: U256 = bnum.into();

        assert_eq!(u256, u256_converted);
        assert_eq!(bnum, bnum_converted);
    }
}
