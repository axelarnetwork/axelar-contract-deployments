//! Message payload account handling with flexible mutability.
//!
//! This module provides a data structure and utilities for working with message payload
//! accounts that can be accessed either mutably or immutably. The payload data is kept
//! as references to avoid copying large amounts of data to program's limited heap.
//!
//! # Memory and Resource Considerations
//!
//! Since the `raw_payload` field is a reference to a potentially large slice of bytes,
//! this implementation specifically avoids making any copies to the (limited) heap
//! of the end-user's program. The data needs to remain available for potentially
//! the full lifetime of the end-user program.
//!
//! # Usage Patterns
//!
//! End-user code should typically use the immutable variant (`ImmutMessagePayload`)
//! when working with message payloads, making sure no unnecessary mutable borrows are
//! requested.
//!
//! The mutable variant (`MutMessagePayload`) should be reserved for specific cases
//! where modification of the payload is actually required, such as during message
//! construction or updates within the Gateway crate.

use std::mem::size_of;
use std::ops::Deref;

use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hashv;
use solana_program::msg;
use solana_program::program_error::ProgramError;

/// Type alias for a message payload with mutable references
pub(crate) type MutMessagePayload<'a> = MessagePayload<'a, Mut>;
/// Type alias for a message payload with immutable references
pub type ImmutMessagePayload<'a> = MessagePayload<'a, Immut>;

/// Data layout of the message payloa PDA account.
///
/// This structure can be instantiated with either mutable or immutable references
/// to its fields
pub struct MessagePayload<'a, R>
where
    R: RefType<'a>,
{
    /// The bump that was used to create the PDA
    pub bump: R::Ref<u8>,
    /// Hash of the whole message.
    ///
    /// Calculated when calling the "commit message payload" instruction.
    ///
    /// All zeroes represent the unhashed, uncommitted state.
    pub payload_hash: R::Ref<[u8; 32]>,
    /// The full message payload contents.
    pub raw_payload: R::Ref<[u8]>,
}

/// Trait to abstract over reference mutability
///
/// This trait allows types to be generic over whether they contain mutable or
/// immutable references, requiring a single implementation to handle both cases.
pub trait RefType<'a> {
    /// The concrete reference type (either &'a T or &'a mut T)
    type Ref<T: 'a + ?Sized>: Deref<Target = T>;
}

/// Type marker for immutable references.
pub struct Immut;
impl<'a> RefType<'a> for Immut {
    type Ref<T: 'a + ?Sized> = &'a T;
}

/// Type marker for mutable references.
pub struct Mut;
impl<'a> RefType<'a> for Mut {
    type Ref<T: 'a + ?Sized> = &'a mut T;
}

impl<'a, R: RefType<'a>> MessagePayload<'a, R> {
    /// Prefix bytes
    ///
    /// 1 byte for the bump plus 32 bytes for the payload hash
    const HEADER_SIZE: usize = size_of::<u8>() + size_of::<[u8; 32]>();

    /// Adds the header prefix space  the given offset.
    #[inline]
    pub fn adjust_offset(offset: usize) -> usize {
        offset.saturating_add(Self::HEADER_SIZE)
    }

    /// Returns `true` if the `payload_hash` section have been modified before.
    pub fn committed(&self) -> bool {
        !self.payload_hash.iter().all(|&x| x == 0)
    }

    /// Asserts this message payload account haven't been committed yet
    #[inline]
    pub fn assert_uncommitted(&self) -> ProgramResult {
        if self.committed() {
            msg!("Error: Message payload account data was already committed");
            Err(ProgramError::InvalidAccountData)
        } else {
            Ok(())
        }
    }
}

/// Tries to parse a mutable `MessagePayload` from mutable account data.
impl<'a> TryFrom<&'a mut [u8]> for MessagePayload<'a, Mut> {
    type Error = ProgramError;

    fn try_from(bytes: &'a mut [u8]) -> Result<Self, Self::Error> {
        if bytes.len() <= Self::HEADER_SIZE {
            msg!("Error: Message payload account data is too small");
            return Err(ProgramError::AccountDataTooSmall);
        }

        let (bump_slice, rest) = bytes.split_at_mut(1);
        let (payload_hash_slice, raw_payload) = rest.split_at_mut(32);
        debug_assert_eq!(payload_hash_slice.len(), 32);
        debug_assert!(!raw_payload.is_empty());

        let bump = &mut bump_slice[0];
        // Unwrap: we just checked that the slice bounds fits the expected array size
        let payload_hash = payload_hash_slice.try_into().unwrap();

        Ok(Self {
            bump,
            payload_hash,
            raw_payload,
        })
    }
}

// Mutable-only methods
impl<'a> MessagePayload<'a, Mut> {
    /// Hashes the contents of `raw_payload` and stores it under `payload_hash`.
    pub fn hash_raw_payload_bytes(&mut self) {
        let digest = hashv(&[(self.raw_payload)]);
        self.payload_hash.copy_from_slice(&(digest.to_bytes()))
    }

    /// Write bytes in `raw_payload`.
    pub fn write(&mut self, bytes_in: &[u8], offset: usize) -> ProgramResult {
        // Check: write bounds
        let write_offset = offset.saturating_add(bytes_in.len());
        if self.raw_payload.len() < write_offset {
            msg!(
                "Write overflow: {} < {}",
                self.raw_payload.len(),
                write_offset
            );
            return Err(ProgramError::AccountDataTooSmall);
        }

        // Write the bytes
        self.raw_payload
            .get_mut(offset..write_offset)
            .ok_or(ProgramError::AccountDataTooSmall)?
            .copy_from_slice(bytes_in);

        Ok(())
    }
}

// Immutable only methods
/// Tries to parse an immutable `MessagePayload` from immutable account data.
impl<'a> TryFrom<&'a [u8]> for MessagePayload<'a, Immut> {
    type Error = ProgramError;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if bytes.len() <= Self::HEADER_SIZE {
            msg!("Error: Message payload account data is too small");
            return Err(ProgramError::AccountDataTooSmall);
        }

        let (bump_slice, rest) = bytes.split_at(1);
        let (payload_hash_slice, raw_payload) = rest.split_at(32);
        debug_assert_eq!(payload_hash_slice.len(), 32);
        debug_assert!(!raw_payload.is_empty());

        let bump = &bump_slice[0];
        // Unwrap: we just checked that the slice bounds fits the expected array size
        let payload_hash = payload_hash_slice.try_into().unwrap();

        Ok(Self {
            bump,
            payload_hash,
            raw_payload,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, Fill};
    use sha3::{Digest, Keccak256};

    #[test]
    fn test_parse() {
        let mut account_data = [0u8; 64];
        let mut rng = thread_rng();
        account_data.try_fill(&mut rng).unwrap();
        let message_payload: ImmutMessagePayload<'_> = account_data.as_slice().try_into().unwrap();

        assert_eq!(*message_payload.bump, account_data[0]);
        assert_eq!(*message_payload.payload_hash, account_data[1..33]);
        assert_eq!(*message_payload.raw_payload, account_data[33..]);
    }

    #[test]
    fn test_hash() {
        let mut account_data = [0u8; 64];
        let mut rng = thread_rng();
        account_data.try_fill(&mut rng).unwrap();
        let mut message_payload: MutMessagePayload<'_> =
            account_data.as_mut_slice().try_into().unwrap();

        message_payload.hash_raw_payload_bytes();

        let expected_hash = Keccak256::digest(&message_payload.raw_payload).to_vec();
        assert_eq!(*expected_hash, *message_payload.payload_hash);
        assert_ne!(expected_hash, vec![0u8; 32]); // confidence check
    }
}
