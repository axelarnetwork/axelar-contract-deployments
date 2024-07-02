use rkyv::bytecheck::{self, CheckBytes};
use rkyv::{Archive, Deserialize, Serialize};

use super::ArchivedWeightedSignature;
use crate::hasher::Hasher;
use crate::types::{SignatureVerificationError, WeightedSignature, U256};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Proof {
    pub(crate) signatures: Vec<WeightedSignature>,
    pub(crate) threshold: U256,
    pub(crate) nonce: u64,
}

impl Proof {
    pub fn new(signatures: Vec<WeightedSignature>, threshold: U256, nonce: u64) -> Self {
        Self {
            signatures,
            threshold,
            nonce,
        }
    }
}

impl ArchivedProof {
    /// Returns the same hash of an equivalent `VerifierSet`.
    pub fn signer_set_hash(&self) -> [u8; 32] {
        let mut hasher = Hasher::default();
        self.drive_visitor_for_signer_set_hash(&mut hasher);
        hasher.finalize()
    }

    pub(crate) fn drive_visitor_for_signer_set_hash(
        &self,
        visitor: &mut impl crate::visitor::ArchivedVisitor,
    ) {
        // Follow `ArchivedVisitor::visit_verifier_set` exact steps
        visitor.prefix_length(self.signatures.len());
        for weighted_signature in self.signatures.iter() {
            visitor.visit_public_key(&weighted_signature.pubkey);
            visitor.visit_u256(&weighted_signature.weight);
        }
        visitor.visit_u256(&self.threshold);
        visitor.visit_u64(&self.nonce);
    }

    pub fn validate_for_message(&self, message: &[u8; 32]) -> Result<(), MessageValidationError> {
        let threshold: bnum::types::U256 = (&self.threshold).into();
        let mut total_weight = bnum::types::U256::ZERO;

        for signature in self.signatures.iter() {
            signature.verify(message)?;
            let signer_weight = &signature.weight;
            total_weight = total_weight
                .checked_add(signer_weight.into())
                .ok_or(MessageValidationError::ArithmeticOverflow)?;

            if total_weight >= threshold {
                return Ok(());
            }
        }
        Err(MessageValidationError::InsufficientWeight)
    }

    pub fn signatures(&self) -> &[ArchivedWeightedSignature] {
        &self.signatures
    }
}

#[derive(thiserror::Error, Debug)]
pub enum MessageValidationError {
    #[error(transparent)]
    InvalidSignature(#[from] SignatureVerificationError),
    #[error("Arithmetic overflow when summing weights")]
    ArithmeticOverflow,
    #[error("Insufficient signer weight")]
    InsufficientWeight,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixtures::{
        random_valid_proof_and_message, random_valid_proof_message_and_verifier_set,
    };

    #[test]
    fn valid_proof() {
        let (proof, message) = random_valid_proof_and_message::<32>();
        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };
        assert!(proof.validate_for_message(&message).is_ok())
    }

    #[test]
    fn invalid_proof_insufficient_weight() {
        let (mut proof, message) = random_valid_proof_and_message::<32>();

        // Fixture Proof threshold values are always equal to the sum of signer weights.
        // Let's bump that.
        proof.threshold = bnum::types::U256::ONE
            .checked_add(proof.threshold.into())
            .unwrap()
            .into();

        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(matches!(
            proof.validate_for_message(&message).unwrap_err(),
            MessageValidationError::InsufficientWeight
        ))
    }

    #[test]
    fn invalid_proof_wrong_message() {
        let (proof, mut message) = random_valid_proof_and_message::<32>();
        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        // Flip the first bit of the message.
        message[0] ^= 1;

        assert!(matches!(
            proof.validate_for_message(&message).unwrap_err(),
            MessageValidationError::InvalidSignature(_)
        ))
    }

    #[test]
    fn exact_hash_of_equivalent_signer_set() {
        let (proof, message, verifier_set) = random_valid_proof_message_and_verifier_set::<32>();

        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(proof.validate_for_message(&message).is_ok()); // Confidence check
        assert_eq!(proof.signer_set_hash(), verifier_set.hash());
    }
}
