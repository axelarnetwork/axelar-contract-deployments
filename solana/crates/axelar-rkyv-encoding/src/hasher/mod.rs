use crate::visitor::{ArchivedVisitor, Visitor};

pub mod generic;

pub mod merkle_trait;
pub mod merkle_tree;
#[cfg(any(test, feature = "test-fixtures", feature = "solana"))]
pub mod solana;

pub trait AxelarRkyv256Hasher<'a>: Default + Visitor<'a> + ArchivedVisitor<'a> {
    fn hash(&mut self, val: &'a [u8]);
    fn hashv(&mut self, vals: &'a [&[u8]]);
    fn result(self) -> Hash256;
}

#[derive(Debug, PartialEq)]
pub struct Hash256(pub [u8; 32]);

impl From<Hash256> for [u8; 32] {
    fn from(value: Hash256) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use generic::Keccak256Hasher;
    use solana::SolanaKeccak256Hasher;

    use super::*;
    use crate::hash_payload;
    use crate::test_fixtures::{random_payload, random_valid_verifier_set};

    const SAMPLE_DATA: [u8; 6] = [0xE2, 0x9B, 0xB0, 0xEF, 0xB8, 0x8F]; // ⛰️

    #[test]
    fn keccak_hasher_output_equals_solana_hasher_without_syscall() {
        let mut hasher = Keccak256Hasher::default();
        hasher.hash(&SAMPLE_DATA);

        let mut solana_hasher = SolanaKeccak256Hasher::default();
        solana_hasher.hash(&SAMPLE_DATA);

        assert_eq!(hasher.result(), solana_hasher.result())
    }

    #[test]
    fn keccak_hasher_output_equals_solana_hasher_without_syscall_on_types() {
        let generic_hasher = Keccak256Hasher::default();
        let solana_hasher = SolanaKeccak256Hasher::default();

        let random_payload = random_payload();
        let random_signer = random_valid_verifier_set();

        let generic_hasher_result =
            hash_payload(&[0; 32], &random_signer, &random_payload, generic_hasher);
        let solana_hasher_result =
            hash_payload(&[0; 32], &random_signer, &random_payload, solana_hasher);

        assert_eq!(generic_hasher_result, solana_hasher_result)
    }
}
