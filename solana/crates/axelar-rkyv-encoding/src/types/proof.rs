use rkyv::{Archive, Deserialize, Serialize};

use crate::hasher::Hasher;
use crate::types::{SignatureVerificationError, WeightedSignature, U256};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct Proof {
    pub(crate) signatures: Vec<WeightedSignature>,
    pub(crate) threshold: U256,
    pub(crate) nonce: u64,
}

impl Proof {
    pub(crate) fn new(signatures: Vec<WeightedSignature>, threshold: U256, nonce: u64) -> Self {
        Self {
            signatures,
            threshold,
            nonce,
        }
    }
}

impl ArchivedProof {
    pub fn signer_set_hash(&self) -> [u8; 32] {
        use crate::visitor::ArchivedVisitor;
        let mut hasher = Hasher::default();
        hasher.tag(b"archived-signer-set");
        hasher.prefix_length(self.signatures.len());
        for signature in self.signatures.iter() {
            ArchivedVisitor::visit_public_key(&mut hasher, &signature.pubkey);
            ArchivedVisitor::visit_u256(&mut hasher, &signature.weight);
        }
        hasher.finalize()
    }

    pub fn validate_for_message(&self, message: &[u8; 32]) -> Result<(), MessageValidationError> {
        let threshold: bnum::types::U256 = (&self.threshold).into();
        let mut total_weight = bnum::types::U256::ZERO;

        // TODO: Optimization: cache the message digest(s) and use
        // `Signer::verify_digest(digest)` (TBD) instead.

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
    use crate::tests::fixtures::random_valid_proof_and_message;

    #[test]
    fn valid_proof() {
        let mut rng = rand::thread_rng();
        let (proof, message) = random_valid_proof_and_message::<32>(&mut rng);
        let serialized = rkyv::to_bytes::<_, 1024>(&proof).unwrap();
        let proof = unsafe { rkyv::archived_root::<Proof>(&serialized) };

        assert!(proof.validate_for_message(&message).is_ok())
    }
}
