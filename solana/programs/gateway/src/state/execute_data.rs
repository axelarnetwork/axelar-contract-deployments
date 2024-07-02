//! Module for the `GatewayExecuteData` account type.

use std::borrow::Cow;

use axelar_rkyv_encoding::types::{
    ArchivedExecuteData, ArchivedMessage, ArchivedProof, ArchivedVerifierSet,
};
use solana_program::hash::hashv;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::processor::ToBytes;

/// Gateway Execute Data type.
/// Represents the execution data for a gateway transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct GatewayExecuteData<'a> {
    /// `rkyv`-archived bytes for the `execute_data` value produced by the
    /// `multisig-prover` contract.
    pub inner: &'a ArchivedExecuteData,

    /// The Keccak256 hash of `Self.inner`.
    pub hash: [u8; 32],

    /// The bump seed for the PDA account.
    pub bump: u8,
}

impl ToBytes for GatewayExecuteData<'_> {
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, GatewayError> {
        Ok(Cow::Borrowed(self.inner.as_bytes()))
    }
}

impl<'a> GatewayExecuteData<'a> {
    /// Creates a new `GatewayExecuteData` struct.
    pub fn new(
        data: &'a [u8],
        gateway_root_pda: &Pubkey,
    ) -> Result<GatewayExecuteData<'a>, GatewayError> {
        let Some(execute_data) = ArchivedExecuteData::from_bytes(data) else {
            solana_program::msg!("Failed to deserialize execute_data bytes");
            return Err(GatewayError::MalformedProof);
        };

        let mut gateway_execute_data = Self {
            inner: execute_data,
            hash: execute_data.proof().signer_set_hash(),
            bump: 0, // bump will be set after we derive the PDA
        };
        let (_pubkey, bump, _seeds) = gateway_execute_data.pda(gateway_root_pda);
        gateway_execute_data.bump = bump;

        Ok(gateway_execute_data)
    }

    /// Returns the seeds for this account PDA.
    pub fn seeds(&self, gateway_root_pda: &Pubkey) -> [u8; 32] {
        hashv(&[
            gateway_root_pda.as_ref(),
            self.hash.as_slice(),
            &[self.bump],
        ])
        .to_bytes()
    }

    /// Finds a PDA for this account. Returns its Pubkey, the canonical bump and
    /// the seeds used to derive them.
    pub fn pda(&self, gateway_root_pda: &Pubkey) -> (Pubkey, u8, [u8; 32]) {
        let seeds = self.seeds(gateway_root_pda);
        let (pubkey, bump) = Pubkey::find_program_address(&[seeds.as_slice()], &crate::ID);
        (pubkey, bump, seeds)
    }

    /// Asserts that the PDA for this account is valid.
    pub fn assert_valid_pda(&self, gateway_root_pda: &Pubkey, exppected_pubkey: &Pubkey) {
        let seeds = self.seeds(gateway_root_pda);
        let derived_pubkey = Pubkey::create_program_address(&[&seeds, &[self.bump]], &crate::ID)
            .expect("invalid bump for the root pda");
        assert_eq!(
            &derived_pubkey, exppected_pubkey,
            "invalid pda for the gateway execute data account"
        );
    }

    /// Returns the archived proof for the internal execute_data value.
    pub fn proof(&self) -> &ArchivedProof {
        self.inner.proof()
    }

    /// Returns the archived message array for the internal execute_data value, if it has one.
    pub fn messages(&self) -> Option<&[ArchivedMessage]> {
        self.inner.messages()
    }

    /// Returns the proposed verifier set for the internal execute_data value, if it has one.
    pub fn verifier_set(&self) -> Option<&ArchivedVerifierSet> {
        self.inner.verifier_set()
    }

    /// Hashes this execute_data payload in the same way it was done over the `multisig-prover` contract.
    pub fn payload_hash(&self, domain_separator: &[u8; 32]) -> [u8; 32] {
        self.inner.internal_payload_hash(domain_separator)
    }
}
