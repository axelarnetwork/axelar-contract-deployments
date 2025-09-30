pub use anchor_discriminators::Discriminator;
use borsh::{BorshDeserialize, BorshSerialize};

// https://github.com/solana-foundation/anchor/blob/18d0ca0ce9b78c03ef370406c6ba86e28e4591ab/lang/src/event.rs#L2
// Sha256(anchor:event)[..8]
#[allow(clippy::unreadable_literal)]
pub const EVENT_IX_TAG: u64 = 0x1d9acb512ea545e4;
#[allow(clippy::little_endian_bytes)]
pub const EVENT_IX_TAG_LE: &[u8] = EVENT_IX_TAG.to_le_bytes().as_slice();

// https://github.com/solana-foundation/anchor/blob/5300d7cf8aaf52da08ce331db3fc8182cd821228/lang/attribute/event/src/lib.rs#L42
pub const SIGHASH_EVENT_NAMESPACE: &str = "event";

pub const EVENT_AUTHORITY_ACCOUNT_NAME: &str = "event_authority";
pub const EVENT_AUTHORITY_SEED: &[u8] = b"__event_authority";

/// An event that can be emitted via a Solana log. See [`emit!`](crate::prelude::emit) for an example.
pub trait CpiEvent: BorshSerialize + BorshDeserialize + Discriminator {
    fn data(&self) -> Vec<u8>;
}
