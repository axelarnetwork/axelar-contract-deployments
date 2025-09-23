//! U256 implementation of uint256.

use std::fmt::Display;

pub use bnum::types::U256 as BnumU256;
use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};

/// [U256] represents uint256.
#[derive(
    Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize, Eq, Copy, Default, Pod, Zeroable,
)]
#[repr(transparent)]
pub struct U256 {
    value: [u64; 4],
}

impl U256 {
    pub const ZERO: U256 = Self { value: [0; 4] };
    pub const ONE: U256 = Self {
        value: [0x01, 0x00, 0x00, 0x00],
    };

    /// Create an integer value from its representation as a byte array in
    /// little endian.
    pub fn from_le_bytes(bytes: [u8; 32]) -> Self {
        let cast: [u64; 4] = bytemuck::cast(bytes);
        Self { value: cast }
    }

    /// const method for initializing u256
    pub const fn from_u64(i: u64) -> Self {
        let mut new_self = Self::ZERO;
        new_self.value[0] = i;
        new_self
    }

    /// Return the memory representation of this integer as a byte array in
    /// little-endian byte order.
    pub fn to_le_bytes(self) -> [u8; 32] {
        let bytes: [u64; 4] = self.value;
        bytemuck::cast(bytes)
    }

    /// Checked integer addition. Computes `self + rhs`, returning `None` if
    /// overflow occurred.
    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        let lhs = bnum::types::U256::from_digits(self.value);
        let rhs = bnum::types::U256::from_digits(rhs.value);

        lhs.checked_add(rhs).map(|res| Self { value: res.into() })
    }

    /// Checked integer subtraction. Computes `self - rhs`, returning `None` if
    /// overflow occurred.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        let lhs = bnum::types::U256::from_digits(self.value);
        let rhs = bnum::types::U256::from_digits(rhs.value);

        lhs.checked_sub(rhs).map(|res| Self { value: res.into() })
    }
}

impl PartialOrd for U256 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for U256 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let lhs = bnum::types::U256::from_digits(self.value);
        let rhs = bnum::types::U256::from_digits(other.value);
        lhs.cmp(&rhs)
    }
}

impl From<u8> for U256 {
    fn from(value: u8) -> Self {
        U256 {
            value: bnum::types::U256::from(value).into(),
        }
    }
}

impl From<u64> for U256 {
    fn from(value: u64) -> Self {
        U256 {
            value: bnum::types::U256::from(value).into(),
        }
    }
}

impl From<usize> for U256 {
    fn from(value: usize) -> Self {
        U256 {
            value: bnum::types::U256::from(value).into(),
        }
    }
}

impl From<u128> for U256 {
    fn from(value: u128) -> Self {
        U256 {
            value: bnum::types::U256::from(value).into(),
        }
    }
}

impl From<&u128> for U256 {
    fn from(value: &u128) -> Self {
        U256 {
            value: bnum::types::U256::from(*value).into(),
        }
    }
}

impl From<U256> for bnum::types::U256 {
    fn from(val: U256) -> Self {
        bnum::types::U256::from(val.value)
    }
}

impl From<U256> for alloy_primitives::U256 {
    fn from(val: U256) -> Self {
        alloy_primitives::U256::from_le_bytes(val.to_le_bytes())
    }
}

impl Display for U256 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let as_bnum = bnum::types::U256::from_digits(self.value);
        f.write_str(&as_bnum.to_string())
    }
}
