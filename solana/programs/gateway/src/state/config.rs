//! Module for the `GatewayConfig` account type.

use std::mem::size_of;
use std::ops::{Deref, DerefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::axelar_auth_weighted::AxelarAuthWeighted;

/// Gateway configuration type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayConfig {
    /// The canonical bump for this account.
    pub bump: u8,
    /// The current set of registered signer set hashes and their epochs.
    pub auth_weighted: AxelarAuthWeighted,
}

impl Deref for GatewayConfig {
    type Target = AxelarAuthWeighted;

    fn deref(&self) -> &Self::Target {
        &self.auth_weighted
    }
}

impl DerefMut for GatewayConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.auth_weighted
    }
}

impl GatewayConfig {
    /// Creates a new `GatewayConfig` value.
    pub fn new(bump: u8, auth_weighted: AxelarAuthWeighted) -> Self {
        Self {
            bump,
            auth_weighted,
        }
    }

    /// Returns the Pubkey and canonical bump for this account.
    pub fn pda() -> (Pubkey, u8) {
        crate::get_gateway_root_config_pda()
    }
}

impl Sealed for GatewayConfig {}

impl Pack for GatewayConfig {
    const LEN: usize = { size_of::<u8>() + AxelarAuthWeighted::SIZE_WHEN_SERIALIZED };

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
