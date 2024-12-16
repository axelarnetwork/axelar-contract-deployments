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
#[derive(Pod, Zeroable, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
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
    pub _padding: [u8; 7],
}

impl BytemuckedPda for GatewayConfig {}

impl GatewayConfig {
    /// Returns `true` if the current epoch is still considered valid given the
    /// signer retention policies.
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
