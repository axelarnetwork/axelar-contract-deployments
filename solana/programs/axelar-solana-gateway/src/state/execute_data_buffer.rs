//! Types repreesenting the internal layout of the `execute_data` buffer.

use arrayref::mut_array_refs;
use bitflags::bitflags;
use solana_program::msg;
use solana_program::program_error::ProgramError;

use super::signature_verification::{BatchContext, SignatureVerification};
use crate::commands::CommandKind;

/// Expected size for the Payload hash.
pub const HASH_SIZE: usize = 32;

/// How many bytes should be allocated for the `execute_data` buffer account
pub const RESERVED_BUFFER_METADATA_BYTES: usize =
    1 + HASH_SIZE + BatchContext::LEN + SignatureVerification::LEN;

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
    pub payload_hash: &'a mut [u8; HASH_SIZE],

    /// Reference to bytes used to store the message batch context.
    pub batch_context: &'a mut [u8; BatchContext::LEN],

    /// Reference to bytes used for storing the signature verification state
    /// machine.
    pub signature_verification: &'a mut [u8; SignatureVerification::LEN],
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

        // Unwrap: We just checked that the remaining bytes have the expected length.
        let rest: &mut [u8; RESERVED_BUFFER_METADATA_BYTES] = rest.try_into().unwrap();

        // Split the remaining bytes into the designated fields.
        let (metadata_byte, payload_hash, batch_context, signature_verification) = mut_array_refs![
            rest,
            1,
            HASH_SIZE,
            BatchContext::LEN,
            SignatureVerification::LEN
        ];

        // Check: BufferMetadata is valid
        // Unwrap: `metadata_byte` slice has exactly one element.
        let metadata_byte = metadata_byte.get_mut(0).unwrap();
        BufferMetadata::from_bits(*metadata_byte).ok_or(ProgramError::InvalidAccountData)?;

        Ok(BufferLayout {
            raw_execute_data,
            metadata_byte,
            payload_hash,
            batch_context,
            signature_verification,
        })
    }

    /// Returns the [`BufferMetadata`] for this buffer.
    pub fn metadata(&self) -> BufferMetadata {
        // Unwrap: We never persist invalid bitflags
        BufferMetadata::from_bits(*self.metadata_byte).unwrap()
    }

    /// Sets the internal flag for this buffer's command kind.
    pub fn set_command_kind(&mut self, command_kind: CommandKind) {
        // Unwrap: Safe to unwrap because the closure always returns `Ok`.
        self.update_metadata(|meta| {
            meta.insert(command_kind.into());
            Ok(())
        })
        .unwrap();
    }

    /// Updates the buffer's internal flag to indicate that the payload hash has
    /// been computed and saved to the account data.
    pub fn commit_payload_hash(
        &mut self,
        payload_hash: &[u8; HASH_SIZE],
    ) -> Result<(), ProgramError> {
        self.payload_hash.copy_from_slice(payload_hash);
        self.update_metadata(|meta| meta.set_payload_hash())?;
        Ok(())
    }

    fn update_metadata<F>(&mut self, func: F) -> Result<(), BufferMetadataError>
    where
        F: Fn(&mut BufferMetadata) -> Result<(), BufferMetadataError>,
    {
        // Unwrap: We never persist invalid bitflags
        let mut flags = BufferMetadata::from_bits(*self.metadata_byte).unwrap();
        func(&mut flags)?;
        *self.metadata_byte = flags.bits();
        Ok(())
    }

    /// Marks this buffer as finalized.
    pub fn finalize(&mut self) -> Result<(), ProgramError> {
        // Unwrap: We never persist invalid bitflags
        let mut flags = BufferMetadata::from_bits(*self.metadata_byte).unwrap();
        flags.finalize()?;
        *self.metadata_byte = flags.bits();
        Ok(())
    }

    /// Initializes a new [`SignatureVerification`] tracker in the current
    /// buffer.
    pub fn initialize_signature_verification(
        &mut self,
        merkle_root: &[u8; 32],
        batch_context: BatchContext,
    ) {
        // Write batch context bytes
        batch_context.serialize_into(self.batch_context);

        // Write signature verification bytes
        let signature_verification = SignatureVerification::new(
            *merkle_root,
            batch_context.signer_count,
            batch_context.threshold,
        );
        signature_verification.serialize_into(self.signature_verification);
    }

    /// Writes the updated [`SignatureVerification`] bytes back in the buffer.
    ///
    /// Consumes the signature verification instance to prevent users from
    /// continuing to use it after it was persisted.
    pub fn update_signature_verification(&mut self, signature_verification: SignatureVerification) {
        signature_verification.serialize_into(self.signature_verification)
    }
}

bitflags! {
    /// Represents the options for the `execute_data` PDA account buffer.
    #[derive(Eq, PartialEq)]
    pub struct BufferMetadata: u8 {
        /// The command kind contained in the buffer.
        ///
        /// ApproveMessages => 0
        /// RotateSigners   => 1
        const COMMAND_KIND = 1 ;

        /// Payload hash calculation status.
        ///
        /// Calculated     => 1
        /// Not calculated => 0
        const PAYLOAD_HASH = 1 << 1;

        /// Buffer finalization status.
        ///
        /// Finalized     => 1
        /// Not finalized => 0
        const FINALIZED = 1 << 2;


    }
}

impl From<CommandKind> for BufferMetadata {
    fn from(command: CommandKind) -> Self {
        match command {
            CommandKind::ApproveMessage => Self::empty(),
            CommandKind::RotateSigner => Self::COMMAND_KIND,
        }
    }
}

impl BufferMetadata {
    fn set_payload_hash(&mut self) -> Result<(), BufferMetadataError> {
        if self.contains(Self::PAYLOAD_HASH) {
            return Err(BufferMetadataError::PayloadHashAlreadySet);
        }
        self.insert(Self::PAYLOAD_HASH);
        Ok(())
    }

    fn finalize(&mut self) -> Result<(), BufferMetadataError> {
        if !self.contains(Self::PAYLOAD_HASH) {
            return Err(BufferMetadataError::FinalizedWithoutPayloadHash);
        }
        self.insert(Self::FINALIZED);
        Ok(())
    }

    /// Returns true if the `FINALIZED` flag is set.
    pub fn is_finalized(&self) -> bool {
        self.contains(Self::FINALIZED)
    }

    /// Returns true if the `PAYLOAD_HASH` is set.
    pub fn has_payload_hash(&self) -> bool {
        self.contains(Self::PAYLOAD_HASH)
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

/// Defines errors that can occur when performing invalid operations on
/// [`BufferMetadata`].
#[derive(thiserror::Error, Debug)]
enum BufferMetadataError {
    #[error("Cannot finalize buffer metadata without a payload hash")]
    FinalizedWithoutPayloadHash,
    #[error("Payload hash flag has already been set")]
    PayloadHashAlreadySet,
}

impl From<BufferMetadataError> for ProgramError {
    fn from(error: BufferMetadataError) -> Self {
        msg!("Buffer metadata error: {}", error);
        ProgramError::InvalidInstructionData
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_data_buffer_metadata_flags() {
        // Test all possible combinations of the flags (0 to 3)
        for bits in 0..=4u8 {
            let meta = BufferMetadata::from_bits(bits).unwrap();

            // `BufferMetadata::is_finalized` should return `true` if the `FINALIZED` flag
            // is set, and `false` otherwise.
            assert_eq!(
                meta.is_finalized(),
                meta.contains(BufferMetadata::FINALIZED),
                "Method `is_finalized` failed for bits {:02b}",
                bits
            );

            // `BufferMetadata::has_payload_hash()` should return `true` if the
            // `PAYLOAD_HASH` flag is set.
            assert_eq!(
                meta.has_payload_hash(),
                meta.contains(BufferMetadata::PAYLOAD_HASH),
                "Method `has_payload_hash` failed for bits {:02b}",
                bits
            );

            // `BufferMetadata::command_kind()` should return `CommandKind::ApproveMessage`
            // if the `COMMAND_KIND` flag is not set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::ApproveMessage),
                !meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );

            // `BufferMetadata::command_kind()` should return `CommandKind::RotateSigner` if
            // the `COMMAND_KIND` flag is set.
            assert_eq!(
                matches!(meta.command_kind(), CommandKind::RotateSigner),
                meta.contains(BufferMetadata::COMMAND_KIND),
                "Invalid output for `command_kind` method for bits {:02b}",
                bits
            );
        }
    }

    #[test]
    fn test_finalize() {
        let mut meta = BufferMetadata::PAYLOAD_HASH;
        assert!(
            !meta.is_finalized(),
            "buffer metadata should not be finalized yet"
        );
        meta.finalize().unwrap();
        assert!(meta.is_finalized(), "buffer metadata should be finalized");
    }

    #[test]
    fn test_cant_finalize_if_invalid() {
        let mut meta = BufferMetadata::empty();
        assert!(matches!(
            meta.finalize(),
            Err(BufferMetadataError::FinalizedWithoutPayloadHash)
        ))
    }

    #[test]
    fn test_set_payload_hash() {
        let mut meta = BufferMetadata::empty();
        assert!(
            !meta.has_payload_hash(),
            "buffer metadata should not have the payload hash flag set"
        );
        meta.set_payload_hash().unwrap();
        assert!(
            meta.has_payload_hash(),
            "buffer metadata should have the payload hash flag set"
        )
    }

    #[test]
    fn test_cant_set_payload_hash_twice() {
        let mut meta = BufferMetadata::PAYLOAD_HASH;
        assert!(matches!(
            meta.set_payload_hash(),
            Err(BufferMetadataError::PayloadHashAlreadySet),
        ));
    }

    #[test]
    fn test_buffer_metadata_lifecycle() {
        let mut meta = BufferMetadata::COMMAND_KIND;
        meta.set_payload_hash().unwrap();
        meta.finalize().unwrap();
        assert!(
            meta == BufferMetadata::COMMAND_KIND
                | BufferMetadata::PAYLOAD_HASH
                | BufferMetadata::FINALIZED,
        )
    }
}
