//! Module for the `GatewayConfig` account type.

use axelar_message_primitives::U256;
use bytemuck::{Pod, Zeroable};
use program_utils::BytemuckedPda;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Timestamp alias for when the last signer rotation happened
pub type Timestamp = u64;
/// Seconds that need to pass between signer rotations
pub type RotationDelaySecs = u64;
/// Ever-incrementing idx for the signer set
pub type VerifierSetEpoch = U256;

/// Gateway configuration type.
#[repr(C)]
#[allow(clippy::partial_pub_fields)]
#[derive(Pod, Zeroable, Debug, PartialEq, Eq, Clone, Copy)]
pub struct GatewayConfig {
    /// current epoch points to the latest signer set hash
    pub current_epoch: VerifierSetEpoch,
    /// how many n epochs do we consider valid
    pub previous_verifier_set_retention: VerifierSetEpoch,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// timestamp tracking of when the previous rotation happened
    pub last_rotation_timestamp: Timestamp,
    /// The gateway operator.
    pub operator: Pubkey,
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// The canonical bump for this account.
    pub bump: u8,
    /// padding for bump
    _padding: [u8; 7],
}

impl BytemuckedPda for GatewayConfig {}

impl GatewayConfig {
    /// Create a new [`GatewayConfig`].
    #[must_use]
    pub const fn new(
        current_epoch: VerifierSetEpoch,
        previous_verifier_set_retention: VerifierSetEpoch,
        minimum_rotation_delay: RotationDelaySecs,
        last_rotation_timestamp: Timestamp,
        operator: Pubkey,
        domain_separator: [u8; 32],
        bump: u8,
    ) -> Self {
        Self {
            current_epoch,
            previous_verifier_set_retention,
            minimum_rotation_delay,
            last_rotation_timestamp,
            operator,
            domain_separator,
            bump,
            _padding: [0; 7],
        }
    }

    /// Asserts that the given epoch is still valid according to the gateway's verifier set
    /// retention policy.
    ///
    /// The epoch is considered valid if the difference between the current epoch and the given
    /// epoch is less than the `previous_verifier_set_retention` value.
    ///
    /// # Errors
    ///
    /// * Returns [`GatewayError::EpochCalculationOverflow`] if the subtraction of epochs would
    /// overflow
    /// * Returns [`GatewayError::VerifierSetTooOld`] if the epoch difference exceeds the retention
    /// period
    #[track_caller]
    pub fn assert_valid_epoch(&self, epoch: U256) -> Result<(), GatewayError> {
        let current_epoch = self.current_epoch;
        let elapsed = current_epoch
            .checked_sub(epoch)
            .ok_or(GatewayError::EpochCalculationOverflow)?;

        if elapsed >= self.previous_verifier_set_retention {
            return Err(GatewayError::VerifierSetTooOld);
        }
        Ok(())
    }
}
