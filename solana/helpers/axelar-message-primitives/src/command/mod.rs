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

#[cfg(test)]
mod test {
    use super::*;

    /// Tests that the `hash_new_operator_set` function correctly hashes the
    /// inputs.
    ///
    /// ```solidity
    /// function calchash(address[] calldata ops, uint256[] calldata weights, uint256 threshold) public pure returns (bytes32) {
    ///    return keccak256(abi.encode(ops, weights, threshold));
    /// }
    /// // The following params:
    /// // ["0x5B38Da6a701c568545dCfcB03FcB875f56beddC4", "0xF135B9eD84E0AB08fdf03A744947cb089049bd79"]
    /// // [1, 2]
    /// // 111
    /// // should return: 0xebb190854a6341f6dc005fe29f70f286a5dc3bed3d5f811eb8e65bb40f19802a
    /// ```
    #[test]
    #[ignore = "
        Ethereum address is 20 bytes, not 32. On which addresses are we operating on?
        Rresearch: https://github.com/eigerco/axelar-amplifier/blob/0be4b7d0d33303d4eecb7405a894b6304ddcecc2/contracts/multisig/src/key.rs
        "]
    fn correctly_hashes() {
        let operators_and_weights = vec![
            (
                Address::try_from("0x5B38Da6a701c568545dCfcB03FcB875f56beddC4").unwrap(),
                U256::ONE,
            ),
            (
                Address::try_from("0xF135B9eD84E0AB08fdf03A744947cb089049bd79").unwrap(),
                U256::from(8_u8),
            ),
        ];
        let threshold = U256::from(111_u8);
        let expected = [
            0xeb, 0xb1, 0x90, 0x85, 0x4a, 0x63, 0x41, 0xf6, 0xdc, 0x00, 0x5f, 0xe2, 0x9f, 0x70,
            0xf2, 0x86, 0xa5, 0xdc, 0x3b, 0xed, 0x3d, 0x5f, 0x81, 0x1e, 0xb8, 0xe6, 0x5b, 0xb4,
            0x0f, 0x19, 0x80, 0x2a,
        ];
        assert_eq!(
            hash_new_operator_set(operators_and_weights.into_iter(), threshold),
            expected
        );
    }
}
