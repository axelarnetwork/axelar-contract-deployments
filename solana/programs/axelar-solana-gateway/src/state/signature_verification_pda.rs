//! Module for the signature verification session PDA data layout type.

use std::mem;

use bytemuck::{Pod, Zeroable};

use super::signature_verification::SignatureVerification;

/// The data layout of a signature verification PDA
///
/// This struct data layout should match the exact account data bytes.
#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone)]
pub struct SignatureVerificationSessionData {
    /// Seed bump for this account's PDA
    pub bump: u8,
    /// [`SignatureVerification`] alignment is 16, so we need to pad.
    _pad: [u8; 15],
    /// Signature verification session
    pub signature_verification: SignatureVerification,
}

impl SignatureVerificationSessionData {
    /// Size, in bytes, to represent a value of this type.
    pub const LEN: usize = mem::size_of::<Self>();
}
