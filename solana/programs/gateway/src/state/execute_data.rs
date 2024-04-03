//! Module for the `GatewayExecuteData` account type.

use axelar_message_primitives::command::{decode, DecodedCommandBatch, Proof};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::hash::hashv;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Gateway Execute Data type.
/// Represents the execution data for a gateway transaction.
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct GatewayExecuteData {
    /// The proof of the transaction, which includes the necessary signatures
    /// and other cryptographic evidence to verify the transaction's validity.
    pub proof: Proof,

    /// The batch of commands that the transaction intends to execute.
    /// These commands are decoded and ready to be processed by the gateway.
    pub command_batch: DecodedCommandBatch,

    /// The hash of the command batch, used to ensure the integrity of the
    /// commands being executed.
    pub command_batch_hash: [u8; 32],

    /// The bump seed for the PDA account.
    pub bump: u8,
}

impl GatewayExecuteData {
    /// Creates a new `GatewayExecuteData` struct.
    pub fn new(data: &[u8], gateway_root_pda: &Pubkey) -> Result<Self, GatewayError> {
        let (proof, command_batch, command_batch_hash) =
            decode(data).map_err(|_| GatewayError::MalformedProof)?;

        let mut gateway_execute_data = Self {
            proof,
            command_batch,
            command_batch_hash,
            bump: 0,
        };
        let (_pubkey, bump, _seeds) = gateway_execute_data.pda(gateway_root_pda);
        // We need to set the bump seed after we have the PDA
        gateway_execute_data.bump = bump;

        Ok(gateway_execute_data)
    }

    /// Returns the seeds for this account PDA.
    pub fn seeds(&self, gateway_root_pda: &Pubkey) -> [u8; 32] {
        hashv(
            [
                gateway_root_pda.as_ref(),
                self.command_batch_hash.as_slice(),
                self.proof.signature_hash().as_slice(),
                self.proof.operators.hash().as_slice(),
            ]
            .as_ref(),
        )
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
}
