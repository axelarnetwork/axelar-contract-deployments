//! Module for dedicated types.

pub mod address;
pub mod bimap;
pub mod execute_data_decoder;
pub mod operator;
pub mod proof;
pub mod pubkey_wrapper;
pub mod signature;
pub mod u256;

use itertools::Itertools;
pub use pubkey_wrapper::PubkeyWrapper;
use solana_program::hash::Hasher;

use self::address::Address;
use self::u256::U256;

/// Hashes the inputs for a new operator set.
pub fn hash_new_operator_set<I>(operators_and_weights: I, threshold: U256) -> [u8; 32]
where
    I: Iterator<Item = (Address, U256)>,
{
    let mut hasher = Hasher::default();
    let operators_and_weights = operators_and_weights
        .sorted_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for (operator, weight) in operators_and_weights {
        hasher.hash(operator.as_ref());
        hasher.hash(&weight.to_le_bytes());
    }
    hasher.hash(&threshold.to_le_bytes());
    hasher.result().to_bytes()
}
