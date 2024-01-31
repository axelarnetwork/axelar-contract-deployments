//! Module for the `GatewayConfig` account type.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

use crate::accounts::discriminator::{Config, Discriminator};
use crate::types::bimap::OperatorsAndEpochs;

/// Gateway configuration type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayConfig {
    discriminator: Discriminator<Config>,
    version: u8,
    /// The current set of registered operators hashes and their epochs.
    pub operators_and_epochs: OperatorsAndEpochs,
}

impl GatewayConfig {
    /// Creates a new `GatewayConfig` value.
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

    /// Reallocate space to store the `GatewayConfig` data.
    #[inline]
    pub fn reallocate<'a>(
        &self,
        config_account: &solana_program::account_info::AccountInfo<'a>,
        payer_account: &solana_program::account_info::AccountInfo<'a>,
        system_account: &solana_program::account_info::AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let data = borsh::to_vec(self)?;
        let size = data.len();
        let new_minimum_balance = Rent::get()?.minimum_balance(size);
        let lamports_diff = new_minimum_balance.saturating_sub(config_account.lamports());
        invoke(
            &system_instruction::transfer(payer_account.key, config_account.key, lamports_diff),
            &[
                payer_account.clone(),
                config_account.clone(),
                system_account.clone(),
            ],
        )?;
        config_account.realloc(size, false)?;
        config_account.try_borrow_mut_data()?[..size].copy_from_slice(&data);
        Ok(())
    }
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            discriminator: Discriminator::new(),
            version: 0,
            operators_and_epochs: OperatorsAndEpochs::default(),
        }
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
