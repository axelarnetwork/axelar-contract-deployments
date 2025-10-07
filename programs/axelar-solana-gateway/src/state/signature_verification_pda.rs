//! Module for the signature verification session PDA data layout type.

use anchor_discriminators_macros::account;
use bytemuck::{Pod, Zeroable};
use program_utils::pda::BytemuckedPda;

use super::signature_verification::SignatureVerification;

/// The data layout of a signature verification PDA
///
/// This struct data layout should match the exact account data bytes.
///
/// Ideally, the payload merkle root should be a part of its seeds.
#[repr(C)]
#[account]
#[allow(clippy::partial_pub_fields)]
#[derive(Zeroable, Pod, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct SignatureVerificationSessionData {
    /// Signature verification session
    pub signature_verification: SignatureVerification,
    /// Seed bump for this account's PDA
    pub bump: u8,
    /// Padding for memory alignment.
    _pad: [u8; 15],
}

impl BytemuckedPda for SignatureVerificationSessionData {}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use crate::types::U128;

    use super::*;

    #[test]
    fn test_initialization() {
        let buffer = [0_u8; size_of::<SignatureVerificationSessionData>()];
        let from_pod: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        let default = &SignatureVerificationSessionData::default();
        assert_eq!(from_pod, default);
        assert_eq!(
            from_pod.signature_verification.accumulated_threshold,
            U128::ZERO
        );
        assert_eq!(from_pod.signature_verification.signature_slots, [0_u8; 32]);
        assert!(!from_pod.signature_verification.is_valid());
    }

    #[test]
    fn test_serialization() {
        let mut buffer: [u8; size_of::<SignatureVerificationSessionData>()] =
            [42; size_of::<SignatureVerificationSessionData>()];

        let original_state;

        let updated_state = {
            let deserialized: &mut SignatureVerificationSessionData =
                bytemuck::cast_mut(&mut buffer);
            original_state = *deserialized;
            let new_threshold = deserialized
                .signature_verification
                .accumulated_threshold
                .saturating_add(U128::new(1));
            deserialized.signature_verification.accumulated_threshold = new_threshold;
            *deserialized
        };
        assert_ne!(updated_state, original_state); // confidence check

        let deserialized: &SignatureVerificationSessionData = bytemuck::cast_ref(&buffer);
        assert_eq!(&updated_state, deserialized);
    }
}
