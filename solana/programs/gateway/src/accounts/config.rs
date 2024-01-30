//! Module for the `GatewayConfig` account type.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::accounts::discriminator::{Config, Discriminator};
use crate::types::bimap::OperatorsAndEpochs;

/// Gateway configuration type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayConfig {
    /// TODO: Change this data type to include the Operators sets
    discriminator: Discriminator<Config>,
    version: u8,
    operators_and_epochs: OperatorsAndEpochs,
}

impl GatewayConfig {
    /// Creates a new
    pub fn new(version: u8, operators_and_epochs: OperatorsAndEpochs) -> Self {
        Self {
            discriminator: Discriminator::new(),
            version,
            operators_and_epochs,
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

    #[test]
    fn serialization_roundtrip() -> Result<()> {
        let mut operators_and_epochs = OperatorsAndEpochs::default();
        operators_and_epochs.update([1u8; 32])?;
        operators_and_epochs.update([2u8; 32])?;
        operators_and_epochs.update([3u8; 32])?;
        let config = GatewayConfig::new(255, operators_and_epochs);
        let serialized = borsh::to_vec(&config)?;
        let deserialized: GatewayConfig = borsh::from_slice(&serialized)?;
        assert_eq!(config, deserialized);
        Ok(())
    }
}
