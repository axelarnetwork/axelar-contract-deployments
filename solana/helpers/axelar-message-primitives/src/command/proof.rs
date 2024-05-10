//! Proof types.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::hash;
use thiserror::Error;

use super::{Operators, Signature, U256};

/// Errors that might happen when updating the operator and epocs set.
#[derive(Error, Debug, PartialEq)]
pub enum ProofError {
    #[error("calculation of weight sum overflowed")]
    WeightCalculationOverflow,
    #[error("accumulated weight of signatures is below the required threshold")]
    LowSignaturesWeight,
    #[error("signers do not match the expected operators list")]
    MalformedSigners,
    #[error(transparent)]
    Secp256k1RecoverError(#[from] solana_program::secp256k1_recover::Secp256k1RecoverError),
}

/// [Proof] represents the Prover produced proof.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Proof {
    /// Look at [Operators]
    pub operators: Operators,
    /// Signatures from multisig.
    pub signatures: Vec<Signature>,
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

    /// The operator set hash for this proof.
    pub fn signature_hash(&self) -> [u8; 32] {
        let signatures = self
            .signatures
            .iter()
            .map(|x| {
                let mut bytes = [0; Signature::ECDSA_RECOVERABLE_SIGNATURE_LEN];
                bytes[..Signature::ECDSA_RECOVERABLE_SIGNATURE_LEN - 1]
                    .copy_from_slice(&x.signature);
                bytes[Signature::ECDSA_RECOVERABLE_SIGNATURE_LEN - 1] = x.recovery_id;
                bytes
            })
            .collect::<Vec<_>>();
        let signaturesa = signatures.iter().map(|x| x.as_ref()).collect::<Vec<_>>();
        let result = hash::hashv(signaturesa.as_slice());

        result.to_bytes()
    }

    /// Perform signatures validation with engagement of secp256k1 ECDSA
    /// recover.
    /// Ported code from [here](https://github.com/axelarnetwork/axelar-cgp-solidity/blob/10b89fb19a44fe9e51989b618811ddd0e1a595f6/contracts/auth/AxelarAuthWeighted.sol#L91)
    pub fn validate_signatures(&self, message_hash: &[u8; 32]) -> Result<(), ProofError> {
        let mut weight = U256::ZERO;
        let op_len = self.operators.addresses().len();
        let mut last_visited_operator_position: usize = 0;
        // todo: switch to https://docs.rs/solana-sdk/latest/solana_sdk/secp256k1_instruction/index.html
        for signature in self.signatures() {
            let signer = solana_program::secp256k1_recover::secp256k1_recover(
                message_hash,
                signature.recovery_id,
                &signature.signature,
            )
            .map_err(ProofError::Secp256k1RecoverError)?
            .to_bytes();
            let signer = &signer[..32];

            // Visiting remaining operators to find a match.
            // Direct array access: 'last_visited_operator_position' was obtained after
            // searching the original array, so this is safe.
            let remaining_operators = &self.operators.addresses()[last_visited_operator_position..];

            // Find a matching operator for this signer or move to the next.
            let Some(operator_index) = remaining_operators
                .iter()
                .enumerate()
                .find(|(_, op_addr)| op_addr.omit_prefix() == signer)
                .map(|(operator_index, _match)| operator_index + last_visited_operator_position)
            else {
                continue;
            };

            // checking if we are out of operators
            if operator_index == op_len {
                return Err(ProofError::MalformedSigners);
            }

            // Accumulate weight.
            weight = weight
                // Direct array access: We got the 'operator_index' after searching the original
                // array, so this is safe.
                .checked_add(self.operators.weights()[operator_index])
                .ok_or(ProofError::WeightCalculationOverflow)?;

            // Check if there is sufficient weight to consider this proof valid.
            if weight >= *self.operators.threshold() {
                return Ok(());
            }

            // Update last visited operator position.
            last_visited_operator_position += 1;
        }

        Err(ProofError::LowSignaturesWeight)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::Address;

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
        let deserialized = borsh::from_slice::<Proof>(&serialized)?;
        let calculated_hash = proof.operators_hash();

        let fixture_message_hash = [
            162, 123, 43, 143, 16, 89, 152, 210, 108, 173, 11, 67, 233, 91, 55, 31, 97, 47, 16, 9,
            227, 16, 111, 118, 201, 227, 137, 219, 64, 133, 4, 115,
        ];
        assert_eq!(calculated_hash, fixture_message_hash);
        assert_eq!(proof, deserialized);
        Ok(())
    }
}
