//! Proof types.

use borsh::{BorshDeserialize, BorshSerialize};

use crate::error::GatewayError;
use crate::types::operator::Operators;
use crate::types::signature::Signature;
use crate::types::u256::U256;

/// [Proof] represents the Prover produced proof.
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq)]
pub struct Proof {
    /// Look at [Operators]
    pub operators: Operators,
    /// Signatures from multisig.
    /// len 65 due to prepended recovery id.
    signatures: Vec<Signature>,
}

impl Proof {
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

    /// The operator set hash for this proof.
    pub fn operators_hash(&self) -> [u8; 32] {
        self.operators.hash()
    }

    /// Perform signatures validation with engagement of secp256k1 recovery
    /// similarly to ethereum ECDSA recovery.
    // TODO: This function's iteration algorithm is overly complex because it
    // operates on the serialized/packed representation for the `Operator` type.
    // We could refactor that original type into a more ergonomic struct to simplify
    // iteration/validation.
    pub fn validate(&self, message_hash: &[u8; 32]) -> Result<(), GatewayError> {
        let mut weight = U256::from_le_bytes([0; 32]);
        let mut last_visited_operator_position: usize = 0;
        for signature in self.signatures() {
            let public_key = signature.sol_recover_public_key(message_hash)?.to_bytes();
            let signer = &public_key[..32];

            // Visiting remaining operators to find a match.
            // Direct array access: 'last_visited_operator_position' was obtained after
            // searching the original array, so this is safe.
            let remaining_operators = &self.operators.addresses()[last_visited_operator_position..];

            if remaining_operators.is_empty() {
                // There are no more operators to look up to.
                // TODO: use a more descriptive error name
                return Err(GatewayError::OperatorsExhausted);
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
                // Direct array access: We got the 'operator_index' after searching the original
                // array, so this is safe.
                .checked_add(self.operators.weights()[operator_index])
                .ok_or(GatewayError::ArithmeticOverflow)?;

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
            Err(GatewayError::AllSignersInvalid)
        } else {
            Err(GatewayError::LowSignaturesWeight)
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
        let address_1 = Address::from([1; 33]);
        let address_2 = Address::from([2; 33]);

        let weight_1 = U256::from_le_bytes([1u8; 32]);
        let weight_2 = U256::from_le_bytes([2u8; 32]);
        let threshold = U256::from_le_bytes([3u8; 32]);

        let operators = Operators::new(
            vec![address_1, address_2],
            vec![weight_1, weight_2],
            threshold,
        );

        let signature_1 = Signature::try_from(vec![0u8; 65])?;
        let signature_2 = Signature::try_from(vec![1u8; 65])?;

        let proof = Proof::new(operators, vec![signature_1, signature_2]);
        let serialized = borsh::to_vec(&proof)?;
        let deserialized: Proof = borsh::from_slice(&serialized)?;

        assert_eq!(proof, deserialized);
        Ok(())
    }
}
