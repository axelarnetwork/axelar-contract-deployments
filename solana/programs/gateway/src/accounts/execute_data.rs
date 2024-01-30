//! Module for the `GatewayExecuteData` account type.

use auth_weighted::types::proof::Proof;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;

use crate::accounts::discriminator::{Discriminator, ExecuteData};
use crate::error::GatewayError;
use crate::types::execute_data_decoder::{decode as decode_execute_data, DecodedCommandBatch};

/// Gateway Execute Data type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayExecuteData {
    discriminator: Discriminator<ExecuteData>,
    /// This is the value produced by Axelar Proover
    data: Vec<u8>,
}

impl GatewayExecuteData {
    /// Creates a new `GatewayExecuteData` struct.
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            discriminator: Discriminator::new(),
        }
    }

    /// Returns the seeds for this account PDA.
    pub fn seeds(&self) -> [u8; 32] {
        hash(&self.data).to_bytes()
    }

    /// Finds a PDA for this account. Returns its Pubkey, the canonical bump and
    /// the seeds used to derive them.
    pub fn pda(&self) -> (Pubkey, u8, [u8; 32]) {
        let seeds = self.seeds();
        let (pubkey, bump) = Pubkey::find_program_address(&[seeds.as_slice()], &crate::ID);
        (pubkey, bump, seeds)
    }

    /// Decodes the `execute_data` into a Proof and a CommandBatch.

    pub fn decode(&self) -> Result<(Proof, DecodedCommandBatch), GatewayError> {
        decode_execute_data(&self.data).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::accounts::discriminator::DiscriminatorTrait;
    #[test]
    fn execute_data_deserialization() -> Result<()> {
        let mut bytes = vec![];
        bytes.extend_from_slice(ExecuteData::DISCRIMINATOR);
        bytes.extend_from_slice(&borsh::to_vec(&(vec![1u8, 2, 3]))?);
        let state = GatewayExecuteData::try_from_slice(&bytes)?;
        assert_eq!(state.data, vec![1, 2, 3]);
        Ok(())
    }

    #[test]
    fn execute_data_invalid_discriminator() -> Result<()> {
        let mut invalid_bytes = vec![];
        invalid_bytes.extend_from_slice(b"deadbeef");
        invalid_bytes.extend_from_slice(&borsh::to_vec(&(vec![1u8, 2, 3]))?);
        let error = GatewayExecuteData::try_from_slice(&invalid_bytes)
            .unwrap_err()
            .into_inner()
            .unwrap()
            .to_string();
        assert_eq!(error, "Invalid discriminator");
        Ok(())
    }

    #[test]
    fn execute_data_pda() -> Result<()> {
        let execute_data = GatewayExecuteData::new(vec![1, 2, 3]);
        let (expected_pda, bump_seed, seed) = execute_data.pda();
        let actual_pda =
            Pubkey::create_program_address(&[seed.as_ref(), &[bump_seed]], &crate::ID)?;
        assert_eq!(expected_pda, actual_pda);
        Ok(())
    }
}
