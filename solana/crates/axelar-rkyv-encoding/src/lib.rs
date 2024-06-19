use hasher::Hasher;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::Fallible;
use types::{Payload, VerifierSet, WeightedSignature};
use visitor::Visitor;

use crate::types::{ExecuteData, Proof};

mod hasher;
mod tests;
pub mod types;
mod visitor;

/// Encodes the execute_data components using N bytes as scratch space allocated
/// on the heap.
pub fn encode<const N: usize>(
    verifier_set: &VerifierSet,
    signatures: Vec<WeightedSignature>,
    payload: Payload,
) -> Result<Vec<u8>, EncodingError<N>> {
    let threshold = verifier_set.threshold;
    let nonce = verifier_set.created_at;
    let proof = Proof::new(signatures, threshold, nonce);
    let execute_data = ExecuteData::new(payload, proof);
    let archived = rkyv::to_bytes::<_, N>(&execute_data).map_err(EncodingError::Serialize)?;
    Ok(archived.into_vec())
}

pub fn hash_payload(
    domain_separator: &[u8; 32],
    signer: &VerifierSet,
    payload: &Payload,
) -> [u8; 32] {
    let mut hasher = Hasher::default();
    hasher.visit_bytes(domain_separator);
    hasher.visit_verifier_set(signer);
    hasher.visit_payload(payload);
    hasher.finalize()
}

#[derive(Debug, thiserror::Error)]
pub enum EncodingError<const N: usize> {
    #[error("Serialization error")]
    Serialize(#[source] <AllocSerializer<N> as Fallible>::Error),
}
