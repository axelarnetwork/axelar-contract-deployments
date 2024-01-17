//! Proof types.

use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::{keccak, msg};

use super::operator::Operators;
use super::signature::Signature;
use super::u256::U256;
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

    /// Perform signatures validation with engagement of secp256k1 recovery
    /// similarly to ethereum ECDSA recovery.
    // TODO: This function's iteration algorithm is overly complex because it
    // operates on the serialized/packed representation for the `Operator` type.
    // We could refactor that original type into a more ergonomic struct to simplify
    // iteration/validation.
    pub fn validate(&self, message_hash: &[u8; 32]) -> Result<(), AuthWeightedError> {
        let mut weight = U256::from_le_bytes([0; 32]);
        let mut last_visited_operator_position: usize = 0;
        for signature in self.signatures() {
            let public_key = signature.sol_recover_public_key(message_hash)?.to_bytes();
            let signer = &public_key[..32];

            // Visiting remaining operators to find a match.
            let remaining_operators = &self.operators.addresses()[last_visited_operator_position..];

            if remaining_operators.is_empty() {
                // There are no more operators to look up to.
                // TODO: use a more descriptive error name
                return Err(AuthWeightedError::MalformedSigners);
            }

            // Find a matching operator for this signer or move to the next.
            let Some((operator_index, _match)) = remaining_operators
                .iter()
                .enumerate()
                .find(|(_, op_addr)| op_addr.omit_prefix() == signer)
            else {
                continue;
            };

            // Update last visited operator position.
            last_visited_operator_position = operator_index;

            // Accumulate weight.
            weight = weight
                .checked_add(*self.operators.weight_by_index(operator_index))
                .ok_or(AuthWeightedError::ArithmeticOverflow)?;

            // Check if there is sufficient weight to consider this proof valid.
            if weight >= *self.operators.threshold() {
                return Ok(());
            }
        }

        // By this point, all operators were visited but there is not enough
        // accumulated weight to consider this proof valid.

        if last_visited_operator_position == 0 {
            // This is specific condition means that not a single operator was matched by
            // any signer.
            Err(AuthWeightedError::MalformedSigners)
        } else {
            Err(AuthWeightedError::LowSignaturesWeight)
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::types::address::Address;
    use crate::types::u256::U256;

    #[test]
    fn test_proof_roundtrip() -> Result<()> {
        let address_1 = Address::try_from(vec![1; 33])?;
        let address_2 = Address::try_from(vec![2; 33])?;

        let weight_1 = U256::from_le_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ]);
        let weight_2 = U256::from_le_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x22,
        ]);

        let threshold = U256::from_le_bytes([
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x20,
        ]);

        let operators = Operators::new(
            vec![address_1, address_2],
            vec![weight_1, weight_2],
            threshold,
        );

        let signature_1 = Signature::try_from(vec![0u8; 65])?;
        let signature_2 = Signature::try_from(vec![1u8; 65])?;

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
        Ok(())
    }
}
