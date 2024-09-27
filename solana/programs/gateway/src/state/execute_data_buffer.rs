//! Types repreesenting the internal layout of the `execute_data` buffer.

use bitflags::bitflags;
use solana_program::program_error::ProgramError;

use crate::commands::CommandKind;

/// Expected size for the Payload hash.
pub const HASH_SIZE: usize = 32;
/// 33 = 1 byte for flags + 32 to store the payload hash
pub const RESERVED_BUFFER_METADATA_BYTES: usize = 1 + HASH_SIZE;

/// Utility struct for managing access to the underlying `execute_data`
/// buffer account data.
pub struct BufferLayout<'a> {
    /// Reference to the raw `execute_data` bytes.
    pub raw_execute_data: &'a mut [u8],
    /// Metadata bits.
    ///
    /// Gated because it needs to be parsed into a [`BufferMetadata`] value
    /// to mutate internally.
    metadata_byte: &'a mut u8,
    /// The calculated hash for the underlying `Payload`.
    ///
    /// It's set to all zeroes until calculated.
    pub payload_hash: &'a mut [u8; 32],
}

impl<'a> BufferLayout<'a> {
    /// Try to parse this buffer from a mutable slice of bytes.
    pub fn parse(data: &'a mut [u8]) -> Result<BufferLayout<'a>, ProgramError> {
        let split_position = data
            .len()
            .checked_sub(RESERVED_BUFFER_METADATA_BYTES)
            .ok_or(ProgramError::AccountDataTooSmall)?;

        // Panic: We just checked that the split position is valid.
        let (raw_execute_data, rest) = data.split_at_mut(split_position);

        // Unwrap: We just checked that `rest` is larger than one byte.
        let (metadata_byte, payload_hash) = rest.split_first_mut().unwrap();

        // Unwrap: we just checked this slice has 32 bytes.
        let payload_hash: &mut [u8; 32] = payload_hash.try_into().unwrap();

        // Check: BufferMetadata is valid
        BufferMetadata::from_bits(*metadata_byte).ok_or(ProgramError::InvalidAccountData)?;

        Ok(BufferLayout {
            raw_execute_data,
            metadata_byte,
            payload_hash,
        })
    }

    /// Returns the [`BufferMetadata`] for this buffer.
    pub fn metadata(&self) -> BufferMetadata {
        // Unwrap: We never persist invalid bitflags
        BufferMetadata::from_bits(*self.metadata_byte).unwrap()
    }

    /// Sets the internal flag for this buffer's command kind.
    pub fn set_command_kind(&mut self, command_kind: CommandKind) {
        // Unwrap: We never persist invalid bitflags
        let mut flags = BufferMetadata::from_bits(*self.metadata_byte).unwrap();
        flags.insert(BufferMetadata::new_from_command_kind(command_kind));
        *self.metadata_byte = flags.bits();
    }

    /// Marks this buffer as finalized.
    pub fn finalize(&mut self) {
        // Unwrap: We never persist invalid bitflags
        let mut flags = BufferMetadata::from_bits(*self.metadata_byte).unwrap();
        flags.finalize();
        *self.metadata_byte = flags.bits();
    }
}

bitflags! {
    /// Represents the options for the `execute_data` PDA account buffer.
    #[derive(Eq, PartialEq)]
    pub struct BufferMetadata: u8 {
        /// Buffer finalization status.
        ///
        /// Finalized     => 1
        /// Not finalized => 0
        const FINALIZED = 1;

        /// The command kind contained in the buffer.
        ///
        /// ApproveMessages => 0
        /// RotateSigners   => 1
        const COMMAND_KIND = 1 << 1;
    }
}

impl BufferMetadata {
    fn new_from_command_kind(command_kind: CommandKind) -> Self {
        match command_kind {
            CommandKind::ApproveMessage => Self::empty(),
            CommandKind::RotateSigner => Self::COMMAND_KIND,
        }
    }

    fn finalize(&mut self) {
        self.insert(Self::FINALIZED);
    }

    /// Returns true if the `FINALIZED` flag is set.
    pub fn is_finalized(&self) -> bool {
        self.contains(Self::FINALIZED)
    }

    /// Returns the internal [`CommandKind`] according to the `COMMAND_KIND`
    /// flag.
    pub fn command_kind(&self) -> CommandKind {
        if self.contains(Self::COMMAND_KIND) {
            CommandKind::RotateSigner
        } else {
            CommandKind::ApproveMessage
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_data_buffer_metadata_flags() {
        // Test all possible combinations of the flags (0 to 3)
        for bits in 0..=3u8 {
            let meta = BufferMetadata::from_bits(bits).unwrap();

            // `Self::is_finalized` should return `true` if the `FINALIZED` flag is set, and
            // `false` otherwise.
            assert_eq!(
                meta.is_finalized(),
                meta.contains(BufferMetadata::FINALIZED),
                "Method `is_finalized` failed for bits {:02b}",
                bits
            );

            // `Self::command_kind()` should return `CommandKind::ApproveMessage` if the
            // `COMMAND_KIND` flag is not set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::ApproveMessage),
                !meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );

            // `Self::command_kind()` should return `CommandKind::RotateSigner` if the
            // `COMMAND_KIND` flag is set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::RotateSigner),
                meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );
        }
    }
}
