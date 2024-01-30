//! Module for the `GatewayConfig` account type.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::accounts::discriminator::{Config, Discriminator};

/// Gateway configuration type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayConfig {
    /// TODO: Change this data type to include the Operators sets
    discriminator: Discriminator<Config>,
    version: u8,
}

impl GatewayConfig {
    /// Creates a new
    pub fn new(version: u8) -> Self {
        Self {
            discriminator: Discriminator::new(),
            version,
        }
    }

    /// Returns the Pubkey and canonical bump for this account.
    pub fn pda() -> (Pubkey, u8) {
        crate::find_root_pda()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::accounts::discriminator::DiscriminatorTrait;

    #[test]
    fn config_deserialization() -> Result<()> {
        let mut bytes = vec![];
        bytes.extend_from_slice(Config::DISCRIMINATOR);
        bytes.push(255); // Version
        let state = GatewayConfig::try_from_slice(&bytes)?;
        assert_eq!(state.version, 255);
        Ok(())
    }

    #[test]
    fn config_invalid_discriminator() -> Result<()> {
        let mut invalid_bytes = vec![];
        invalid_bytes.extend_from_slice(b"deadbeef"); // Invalid discriminator
        invalid_bytes.push(1); // Version

        let error = GatewayConfig::try_from_slice(&invalid_bytes)
            .unwrap_err()
            .into_inner()
            .unwrap()
            .to_string();
        assert_eq!(error, "Invalid discriminator");
        Ok(())
    }
}
