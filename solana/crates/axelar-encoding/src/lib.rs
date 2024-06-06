use hasher::PayloadHasher;
use types::{EncodingError, Payload, WeightedSignature, WorkerSet};
use visitor::Visitor;

use crate::types::{ExecuteData, Proof};

mod hasher;
mod tests;
pub mod types;
mod visitor;

/// Encodes the execute_data components using N bytes as scratch space allocated
/// on the heap.
pub fn encode<const N: usize>(
    worker_set: &WorkerSet,
    signatures: Vec<WeightedSignature>,
    payload: Payload,
) -> Result<Vec<u8>, EncodingError<N>> {
    let threshold = worker_set.threshold;
    let nonce = worker_set.created_at;
    let proof = Proof::new(signatures, threshold, nonce);
    let execute_data = ExecuteData::new(payload, proof);
    let archived = rkyv::to_bytes::<_, N>(&execute_data).map_err(EncodingError::Serialize)?;
    Ok(archived.into_vec())
}

pub fn hash_payload(
    domain_separator: &[u8; 32],
    signer: &WorkerSet,
    payload: &Payload,
) -> [u8; 32] {
    let mut hasher = PayloadHasher::default();
    hasher.visit_bytes(domain_separator);
    hasher.visit_worker_set(signer);
    hasher.visit_payload(payload);
    hasher.finalize()
}
