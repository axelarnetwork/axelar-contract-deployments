//! Module for the signature verification session PDA data layout type.

use std::mem;

use bytemuck::{Pod, Zeroable};

use super::signature_verification::SignatureVerification;

/// The data layout of a signature verification PDA
///
/// This struct data layout should match the exact account data bytes.
///
/// Ideally, the payload merkle root should be a part of its seeds.
#[repr(C)]
#[derive(Zeroable, Pod, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct SignatureVerificationSessionData {
    /// Signature verification session
    pub signature_verification: SignatureVerification,
    /// Seed bump for this account's PDA
    pub bump: u8,
    _pad: [u8; 15],
}

impl SignatureVerificationSessionData {
    /// Size, in bytes, to represent a value of this type.
    pub const LEN: usize = mem::size_of::<Self>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        let buffer = [0u8; SignatureVerificationSessionData::LEN];
        let from_pod: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        let default = &SignatureVerificationSessionData::default();
        assert_eq!(from_pod, default);
        assert_eq!(from_pod.signature_verification.accumulated_threshold, 0);
        assert_eq!(from_pod.signature_verification.signature_slots, [0u8; 32]);
        assert!(!from_pod.signature_verification.is_valid())
    }

    #[test]
    fn test_serialization() {
        let mut buffer: [u8; SignatureVerificationSessionData::LEN] =
            [42; SignatureVerificationSessionData::LEN];

        let original_state;

        let updated_state = {
            let deserialized: &mut SignatureVerificationSessionData =
                bytemuck::cast_mut(&mut buffer);
            original_state = *deserialized;
            let (new_threshold, _) = deserialized
                .signature_verification
                .accumulated_threshold
                .overflowing_add(1);
            deserialized.signature_verification.accumulated_threshold = new_threshold;
            *deserialized
        };
        assert_ne!(updated_state, original_state); // confidence check

        let deserialized: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        assert_eq!(&updated_state, deserialized);
    }
}
