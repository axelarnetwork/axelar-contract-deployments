//! Module for the MessagePayload account type.

use std::mem::size_of;

use solana_program::entrypoint::ProgramResult;
use solana_program::keccak::hashv;
use solana_program::msg;
use solana_program::program_error::ProgramError;

/// Data layout of the message payloa PDA account.
pub struct MessagePayload<'a> {
    /// The bump that was used to create the PDA
    pub bump: &'a mut u8,
    /// Hash of the whole message.
    ///
    /// Calculated when calling the "commit message payload" instruction.
    ///
    /// All zeroes represent the unhashed, uncommitted state.
    pub payload_hash: &'a mut [u8; 32],
    /// The full message payload contents.
    pub raw_payload: &'a mut [u8],
}

impl<'a> MessagePayload<'a> {
    /// Prefix bytes
    ///
    /// 1 byte for the bump plus 32 bytes for the payload hash
    const HEADER_SIZE: usize = size_of::<u8>() + size_of::<[u8; 32]>();

    /// Adds the header prefix space  the given offset.
    #[inline]
    pub fn adjust_offset(offset: usize) -> usize {
        offset.saturating_add(Self::HEADER_SIZE)
    }

    /// Tries to parse a `MessagePayload` from the contents of a borrowed account data slice.
    pub fn from_borrowed_account_data(account_data: &'a mut [u8]) -> Result<Self, ProgramError> {
        if account_data.len() <= Self::HEADER_SIZE {
            msg!("Error: Message payload account data is too small");
            return Err(ProgramError::InvalidAccountData);
        }

        let (bump_slice, rest) = account_data.split_at_mut(1);
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

    /// Hashes the contents of `raw_payload` and stores it under `payload_hash`.
    pub fn hash_raw_payload_bytes(&mut self) {
        let digest = hashv(&[(self.raw_payload)]);
        self.payload_hash.copy_from_slice(&(digest.to_bytes()))
    }

    /// Returns `true` if the `payload_hash` section have been modified before.
    pub fn committed(&self) -> bool {
        !self.payload_hash.iter().all(|&x| x == 0)
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
        let mut copy = account_data;
        let message_payload = MessagePayload::from_borrowed_account_data(&mut copy).unwrap();

        assert_eq!(*message_payload.bump, account_data[0]);
        assert_eq!(*message_payload.payload_hash, account_data[1..33]);
        assert_eq!(*message_payload.raw_payload, account_data[33..]);
    }

    #[test]
    fn test_hash() {
        let mut account_data = [0u8; 64];
        let mut rng = thread_rng();
        account_data.try_fill(&mut rng).unwrap();
        let mut message_payload =
            MessagePayload::from_borrowed_account_data(&mut account_data).unwrap();

        message_payload.hash_raw_payload_bytes();

        let expected_hash = Keccak256::digest(&message_payload.raw_payload).to_vec();
        assert_eq!(*expected_hash, *message_payload.payload_hash);
        assert_ne!(expected_hash, vec![0u8; 32]); // confidence check
    }
}
