//! Module for the `GatewayConfig` account type.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

use crate::error::GatewayError;
use crate::state::discriminator::{Config, Discriminator};
use crate::state::transfer_operatorship::sorted_and_unique;
use crate::types::bimap::OperatorsAndEpochs;
use crate::types::operator::Operators;
use crate::types::u256::U256;

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
        crate::get_gateway_root_config_pda()
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

    /// Validate if the given operator set is registered.
    pub(crate) fn validate_proof_operators(
        &self,
        operators: &Operators,
    ) -> Result<(), GatewayError> {
        // Number of recent operator sets to be tracked.
        const OLD_KEY_RETENTION: u8 = 16;

        // TODO: The following checks are equal to the ones in the transfer operatorship
        // account. We should encapsulate that logic into a function.
        // Check: non-empty operator list.
        if operators.addresses().is_empty() {
            return Err(GatewayError::EmptyOperators);
        }

        // Check: threshold is non-zero.
        if *operators.threshold() == U256::ZERO {
            return Err(GatewayError::ZeroThreshold);
        }

        // Check: operator addresses are sorted and are unique.
        if !sorted_and_unique(operators.addresses().iter()) {
            return Err(GatewayError::UnorderedOrDuplicateOperators);
        }

        // TODO Should we not be comparing the weights and operators against the current
        // epoch? TODO: Double check with the Solidity implementation.
        // Check: sufficient threshold.
        let total_weight: U256 = operators
            .weights()
            .iter()
            .try_fold(U256::ZERO, |a, &b| a.checked_add(b))
            .ok_or(GatewayError::ArithmeticOverflow)?;
        if total_weight < *operators.threshold() {
            return Err(GatewayError::InsufficientOperatorWeight);
        }

        // Check: operators are registered
        let operators_epoch = self
            .operators_and_epochs
            .epoch_for_operator_hash(&operators.hash())
            .ok_or(GatewayError::EpochForHashNotFound)?;

        let current_epoch = self.operators_and_epochs.current_epoch();

        let operator_epoch_is_outdated = current_epoch
            .checked_sub(*operators_epoch)
            .ok_or(GatewayError::ArithmeticOverflow)?
            >= U256::from(OLD_KEY_RETENTION);
        if operator_epoch_is_outdated {
            return Err(GatewayError::OutdatedOperatorsEpoch);
        }

        if *operators_epoch == U256::ZERO {
            return Err(GatewayError::EpochZero)?;
        };

        if *operators_epoch != current_epoch {
            return Err(GatewayError::EpochMissmatch)?;
        }
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
