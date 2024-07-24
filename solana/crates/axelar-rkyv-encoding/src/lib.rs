use hasher::AxelarRkyv256Hasher;
use rkyv::ser::serializers::AllocSerializer;
use rkyv::Fallible;
use types::{Payload, PublicKey, VerifierSet, WeightedSigner, U256};
use visitor::Visitor;

use crate::types::{ExecuteData, Proof};

pub mod hasher;
pub mod types;
pub mod visitor;

#[cfg(test)]
mod tests;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_fixtures;

/// Encodes the execute_data components using N bytes as scratch space allocated
/// on the heap.
pub fn encode<const N: usize>(
    created_at: u64,
    threshold: U256,
    signers_with_signatures: Vec<(PublicKey, WeightedSigner)>,
    payload: Payload,
) -> Result<Vec<u8>, EncodingError<N>> {
    let signers_with_signatures = signers_with_signatures.into_iter().collect();
    let proof = Proof::new(signers_with_signatures, threshold, created_at);
    let execute_data = ExecuteData::new(payload, proof);
    let archived = rkyv::to_bytes::<_, N>(&execute_data).map_err(EncodingError::Serialize)?;
    Ok(archived.into_vec())
}

pub fn hash_payload<'a>(
    domain_separator: &'a [u8; 32],
    signer: &'a VerifierSet,
    payload: &'a Payload,
    mut hasher_impl: impl AxelarRkyv256Hasher<'a>,
) -> [u8; 32] {
    Visitor::visit_bytes(&mut hasher_impl, domain_separator);
    Visitor::visit_verifier_set(&mut hasher_impl, signer);
    Visitor::visit_payload(&mut hasher_impl, payload);
    hasher_impl.result().into()
}

#[derive(Debug, thiserror::Error)]
pub enum EncodingError<const N: usize> {
    #[error("Serialization error")]
    Serialize(#[source] <AllocSerializer<N> as Fallible>::Error),
}
