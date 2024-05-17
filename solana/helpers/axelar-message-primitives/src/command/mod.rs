mod decoded_command;
mod operator;
mod proof;
mod signature;
mod u256;

pub use decoded_command::*;
use itertools::Itertools;
pub use operator::*;
pub use proof::*;
pub use signature::*;
use solana_program::hash::Hasher;
pub use u256::*;

use crate::Address;

/// Hashes the inputs for a new operator set.
pub fn hash_new_operator_set<'a, I, K>(operators_and_weights: I, threshold: U256) -> [u8; 32]
where
    I: Iterator<Item = (&'a Address, K)>,
    K: Into<U256>,
{
    let mut hasher = Hasher::default();
    let operators_and_weights = operators_and_weights
        .sorted_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for (operator, weight) in operators_and_weights {
        hasher.hash(operator.as_ref());
        hasher.hash(&weight.into().to_le_bytes());
    }
    hasher.hash(&threshold.to_le_bytes());
    hasher.result().to_bytes()
}
