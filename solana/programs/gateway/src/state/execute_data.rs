//! Module for the `GatewayExecuteData` account type.

use std::borrow::Cow;

use axelar_rkyv_encoding::types::{
    ArchivedExecuteData, ArchivedMessage, ArchivedProof, ArchivedVerifierSet,
};
use solana_program::hash::hashv;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::hasher_impl;
use crate::processor::ToBytes;

/// Gateway Execute Data type.
/// Represents the execution data for a gateway transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct GatewayExecuteData<'a> {
    /// `rkyv`-archived bytes for the `execute_data` value produced by the
    /// `multisig-prover` contract.
    pub inner: &'a ArchivedExecuteData,

    /// Pre-computed message payload hash
    pub payload_hash: [u8; 32],

    /// The bump seed for the PDA account.
    pub bump: u8,

    /// The original bytes that form `Self.inner`
    original_execute_data: &'a [u8],
}

impl ToBytes for GatewayExecuteData<'_> {
    fn to_bytes(&self) -> Result<Cow<'_, [u8]>, GatewayError> {
        Ok(Cow::Borrowed(self.original_execute_data))
    }
}

impl<'a> GatewayExecuteData<'a> {
    /// Creates a new `GatewayExecuteData` struct.
    pub fn new(
        data: &'a [u8],
        gateway_root_pda: &Pubkey,
        domain_separator: &[u8; 32],
    ) -> Result<GatewayExecuteData<'a>, GatewayError> {
        let execute_data = match ArchivedExecuteData::from_bytes(data) {
            Ok(execute_data) => execute_data,
            Err(err) => {
                solana_program::msg!("Failed to deserialize execute_data bytes {:?}", err);
                return Err(GatewayError::MalformedProof);
            }
        };

        let payload_hash = execute_data.internal_payload_hash(domain_separator, hasher_impl());
        let mut gateway_execute_data = Self {
            inner: execute_data,
            payload_hash,
            bump: 0, // bump will be set after we derive the PDA
            original_execute_data: data,
        };
        let (_pubkey, bump, _seeds) = gateway_execute_data.pda(gateway_root_pda);
        gateway_execute_data.bump = bump;

        Ok(gateway_execute_data)
    }

    /// Returns the seeds for this account PDA.
    pub fn seeds(&self, gateway_root_pda: &Pubkey) -> [u8; 32] {
        hashv(&[
            gateway_root_pda.as_ref(),
            self.inner.hash(hasher_impl()).as_slice(),
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
    pub fn assert_valid_pda(&self, gateway_root_pda: &Pubkey, expected_pubkey: &Pubkey) {
        let (derived_pubkey, _bump, _seeds) = self.pda(gateway_root_pda);
        assert_eq!(
            &derived_pubkey, expected_pubkey,
            "invalid pda for the gateway execute data account"
        );
    }

    /// Returns the archived proof for the internal execute_data value.
    pub fn proof(&self) -> &ArchivedProof {
        self.inner.proof()
    }

    /// Returns the archived message array for the internal execute_data value,
    /// if it has one.
    pub fn messages(&self) -> Option<&[ArchivedMessage]> {
        self.inner.messages()
    }

    /// Returns the proposed verifier set for the internal execute_data value,
    /// if it has one.
    pub fn verifier_set(&self) -> Option<&ArchivedVerifierSet> {
        self.inner.verifier_set()
    }
}

#[test]
fn test_gateway_execute_data_roundtrip() {
    use axelar_rkyv_encoding::test_fixtures::random_valid_execute_data_and_verifier_set;
    let domain_separator = [5; 32];
    let gateway_root_pda = Pubkey::new_unique();
    let (execute_data, _) = random_valid_execute_data_and_verifier_set(&domain_separator);
    let raw_data = execute_data.to_bytes::<0>().unwrap();

    let gateway_execute_data =
        GatewayExecuteData::new(&raw_data, &gateway_root_pda, &domain_separator).unwrap();
    let serialized_gateway_execute_data = ToBytes::to_bytes(&gateway_execute_data).unwrap();

    assert_eq!(*serialized_gateway_execute_data, *raw_data);
}
