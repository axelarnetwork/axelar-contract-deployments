//! Module for the Gateway program account structs.

pub mod config;
pub mod incoming_message;
pub mod signature_verification;
pub mod signature_verification_pda;
pub mod verifier_set_tracker;

use bytemuck::{AnyBitPattern, NoUninit};
pub use config::GatewayConfig;
use solana_program::msg;

use crate::error::GatewayError;

/// A trait for types that can be safely converted to and from byte slices using `bytemuck`.
pub trait BytemuckedPda: Sized + NoUninit + AnyBitPattern {
    /// Reads an immutable reference to `Self` from a byte slice.
    ///
    /// This method attempts to interpret the provided byte slice as an instance of `Self`.
    /// It checks that the length of the slice matches the size of `Self` to ensure safety.
    fn read(data: &[u8]) -> Result<&Self, GatewayError> {
        let result: &Self = bytemuck::try_from_bytes(data).map_err(|err| {
            msg!("bytemuck error {:?}", err);
            GatewayError::BytemuckDataLenInvalid
        })?;
        Ok(result)
    }

    /// Reads a mutable reference to `Self` from a mutable byte slice.
    ///
    /// Similar to [`read`], but allows for mutation of the underlying data.
    /// This is useful when you need to modify the data in place.
    fn read_mut(data: &mut [u8]) -> Result<&mut Self, GatewayError> {
        let result: &mut Self = bytemuck::try_from_bytes_mut(data).map_err(|err| {
            msg!("bytemuck error {:?}", err);
            GatewayError::BytemuckDataLenInvalid
        })?;
        Ok(result)
    }

    /// Writes the instance of `Self` into a mutable byte slice.
    ///
    /// This method serializes `self` into its byte representation and copies it into the
    /// provided mutable byte slice. It ensures that the destination slice is of the correct
    /// length to hold the data.
    fn write(&self, data: &mut [u8]) -> Result<(), GatewayError> {
        let self_bytes = bytemuck::bytes_of(self);
        if data.len() != self_bytes.len() {
            return Err(GatewayError::BytemuckDataLenInvalid);
        }
        data.copy_from_slice(self_bytes);
        Ok(())
    }
}
