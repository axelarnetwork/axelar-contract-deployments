//! Proof types.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::keccak;

use super::operator::Operators;
use super::signature::Signature;
use crate::error::AuthWeightedError;

/// [Proof] represents the Prover produced proof.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct Proof {
    /// Look at [Operators]
    pub operators: Operators,
    /// Signatures from multisig.
    /// len 65 due to prepended recovery id.
    signatures: Vec<Signature>,
}

impl<'a> Proof {
    /// Constructor for [Proof].
    pub fn new(operators: Operators, signatures: Vec<Signature>) -> Self {
        Self {
            operators,
            signatures,
        }
    }

    /// Returns vector of signatures.
    pub fn signatures(&self) -> &Vec<Signature> {
        &self.signatures
    }

    /// Deserialize [Proof].
    pub fn unpack(input: &'a [u8]) -> Result<Self, AuthWeightedError> {
        match Self::try_from_slice(input) {
            Ok(v) => Ok(v),
            Err(_) => Err(AuthWeightedError::InvalidInstruction),
        }
    }

    /// Serialize [Proof].
    pub fn pack(&self) -> Vec<u8> {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        to_vec(&self).unwrap()
    }

    /// Generate hash of [Operators].
    pub fn get_operators_hash(&self) -> [u8; 32] {
        // It is safe to unwrap here, as to_vec doesn't return Error.
        keccak::hash(&to_vec(&self.operators).unwrap()).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::address::Address;
    use crate::types::u256::U256;

    #[test]
    fn test_proof_roundtrip() {
        let address_1 = Address::new(
            [
                0x04, 0xd9, 0xb5, 0xb5, 0xf2, 0x52, 0x99, 0xc8, 0xa9, 0xa4, 0x0e, 0x4e, 0xd8, 0x5a,
                0x65, 0x47, 0x19, 0xc3, 0x50, 0xfa, 0xf9, 0xf9, 0xc3, 0xa1, 0x7f, 0x2c, 0x6a, 0x74,
                0x7b, 0x98, 0x1d, 0x5b, 0x25, 0x49, 0x54, 0x1b, 0xfa, 0x6e, 0x5c, 0x06, 0xa1, 0x7e,
                0x2b, 0x2f, 0xe1, 0x0c, 0x6a, 0xc4, 0x03, 0xdf, 0x23, 0xc6, 0xe7, 0xef, 0x97, 0xbf,
                0x2f, 0xf8, 0x18, 0xf2, 0x12, 0x63, 0x51, 0x31,
            ]
            .to_vec(),
        );
        let address_2 = Address::new(
            [
                0x04, 0xd9, 0xb5, 0xb5, 0xf2, 0x52, 0x99, 0xc8, 0xa9, 0xa4, 0x0e, 0x4e, 0xd8, 0x5a,
                0x65, 0x47, 0x19, 0xc3, 0x50, 0xfa, 0xf9, 0xf9, 0xc3, 0xa1, 0x7f, 0x2c, 0x6a, 0x74,
                0x7b, 0x98, 0x1d, 0x5b, 0x25, 0x49, 0x54, 0x1b, 0xfa, 0x6e, 0x5c, 0x06, 0xa1, 0x7e,
                0x2b, 0x2f, 0xe1, 0x0c, 0x6a, 0xc4, 0x03, 0xdf, 0x23, 0xc6, 0xe7, 0xef, 0x97, 0xbf,
                0x2f, 0xf8, 0x18, 0xf2, 0x12, 0x63, 0x51, 0x30,
            ]
            .to_vec(),
        );

        let weight_1 = U256::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ]);
        let weight_2 = U256::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x22,
        ]);

        let threshold = U256::new([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x20,
        ]);

        let operators = Operators::new(
            vec![address_1, address_2],
            vec![weight_1, weight_2],
            threshold,
        );

        let input_1 = [1u8; 64].to_vec();
        let input_2 = [3u8; 64].to_vec();

        let signature_1 = Signature::new(input_1);
        let signature_2 = Signature::new(input_2);

        let proof = Proof::new(
            operators.clone(),
            vec![signature_1.clone(), signature_2.clone()],
        );
        let serialized = proof.pack();
        let deserialized = Proof::unpack(&serialized).unwrap();

        let b_proof = Proof::new(
            operators,
            vec![signature_1.clone(), signature_2, signature_1],
        );
        let b_serialized = b_proof.pack();
        let b_deserialized = Proof::unpack(&b_serialized).unwrap();

        assert_eq!(proof, deserialized);
        assert_ne!(proof, b_deserialized);
    }
}
