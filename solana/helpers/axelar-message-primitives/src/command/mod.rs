mod decoded_command;
mod proof;
mod signature;
mod signer_set;
mod u256;

pub use decoded_command::*;
use itertools::Itertools;
pub use proof::*;
pub use signature::*;
pub use signer_set::*;
use solana_program::hash::Hasher;
pub use u256::*;

use crate::Address;

/// Hashes the inputs for a new operator set.
// TODO: This function should work for all the types in axelar-rkyv-encoding
// crate. Ideally, it should also take the NONCE into account.
pub fn hash_new_signer_set<'a, I, K>(signer_set_and_weights: I, threshold: U256) -> [u8; 32]
where
    I: Iterator<Item = (&'a Address, K)>,
    K: Into<U256>,
{
    let mut hasher = Hasher::default();
    let signer_set_and_weights = signer_set_and_weights
        .sorted_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for (signer, weight) in signer_set_and_weights {
        hasher.hash(signer.as_ref());
        hasher.hash(&weight.into().to_le_bytes());
    }
    hasher.hash(&threshold.to_le_bytes());
    hasher.result().to_bytes()
}
