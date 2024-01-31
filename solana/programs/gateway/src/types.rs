//! Module for dedicated types.

pub mod address;
pub mod bimap;
pub mod execute_data_decoder;
pub mod operator;
pub mod proof;
pub mod pubkey_wrapper;
pub mod u256;

pub use pubkey_wrapper::PubkeyWrapper;
use solana_program::hash::Hasher;

use self::address::Address;
use self::u256::U256;

/// Hashes the inputs for a new operator set.
pub fn hash_new_operator_set(
    operators_and_weights: &[(Address, U256)],
    threshold: U256,
) -> [u8; 32] {
    let mut hasher = Hasher::default();
    for (operator, weight) in operators_and_weights {
        hasher.hash(operator.as_ref());
        hasher.hash(&weight.to_le_bytes());
    }
    hasher.hash(&threshold.to_le_bytes());
    hasher.result().to_bytes()
}
