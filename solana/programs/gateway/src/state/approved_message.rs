//! Module for the GatewayApprovedMessage account type.

use std::mem::size_of;
use std::ops::Deref;

use axelar_message_primitives::{AxelarMessageParams, CommandId, DestinationProgramId};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;
use crate::state::discriminator::{Discriminator, MessageID};

/// Possible statuses for a [`GatewayApprovedMessage`].
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum MessageApprovalStatus {
    /// Message pending for approval
    Pending,
    /// Message was approved
    Approved,
    /// Message has been executed
    Executed,
}

/// Gateway Approved Message type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayApprovedMessage {
    discriminator: Discriminator<MessageID>,
    /// Status of the message
    pub status: MessageApprovalStatus,
    /// The pubkey of the account that is allowed to execute this message
    pub bump: u8,
}

impl GatewayApprovedMessage {
    /// Returns an approved message.
    pub fn pending(bump: u8) -> Self {
        Self {
            discriminator: Discriminator::new(),
            status: MessageApprovalStatus::Pending,
            bump,
        }
    }

    /// Makes sure that the attached account info is the expected one
    /// If successful verification: will update the status to `Executed`
    pub fn verify_caller(
        &mut self,
        message_id: &CommandId,
        destination_pubkey: &DestinationProgramId,
        caller: &AccountInfo<'_>,
    ) -> Result<(), ProgramError> {
        let (allowed_caller, _allowed_caller_bump) = destination_pubkey.signing_pda(message_id);
        if allowed_caller != *caller.key {
            return Err(GatewayError::MismatchedAllowedCallers.into());
        }

        if !caller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !self.is_approved() {
            return Err(GatewayError::GatewayMessageNotApproved.into());
        }

        self.status = MessageApprovalStatus::Executed;

        Ok(())
    }

    /// Sets the message status as approved.
    pub fn set_approved(&mut self) {
        self.status = MessageApprovalStatus::Approved;
    }

    /// Returns `true` if this message was approved.
    pub fn is_approved(&self) -> bool {
        matches!(self.status, MessageApprovalStatus::Approved)
    }

    /// Returns `true` if this message is still waiting for approval.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, MessageApprovalStatus::Pending)
    }

    /// Returns `true` if this message has been executed.
    pub fn is_executed(&self) -> bool {
        matches!(self.status, MessageApprovalStatus::Executed)
    }

    /// Finds a PDA for this account by hashing the parameters. Returns its
    /// Pubkey and bump.
    pub fn pda(
        gateway_root_pda: &Pubkey,
        message_params: &AxelarMessageParams<'_>,
    ) -> (Pubkey, u8, [u8; 32]) {
        let seeds_hash = Self::calculate_seed_hash(gateway_root_pda, message_params);

        let (pubkey, bump) = Pubkey::find_program_address(&[seeds_hash.as_ref()], &crate::ID);
        (pubkey, bump, seeds_hash)
    }

    /// Calculates the seed hash for the PDA of this account.
    pub fn calculate_seed_hash(
        gateway_root_pda: &Pubkey,
        message_params: &AxelarMessageParams<'_>,
    ) -> [u8; 32] {
        use solana_program::keccak::hashv;

        let (signing_pda_for_destination_pubkey, signing_pda_bump) = message_params
            .destination_program
            .signing_pda(&message_params.command_id);

        // TODO we should replace the gateway root pda with the execute data pda.
        //      It intrinsicly would depend on the gateway pda + would link with the
        // expected execute data acc
        let seeds = &[
            gateway_root_pda.as_ref(),
            message_params.command_id.0.as_ref(),
            message_params.source_chain.0.as_bytes(),
            message_params.source_address.0,
            message_params.payload_hash.0.deref(),
            signing_pda_for_destination_pubkey.as_ref(),
            &[signing_pda_bump],
        ];

        // Hashing is necessary because otherwise the seeds would be too long

        hashv(seeds).to_bytes()
    }
}

impl Sealed for GatewayApprovedMessage {}

impl Pack for GatewayApprovedMessage {
    const LEN: usize = 9 + size_of::<Pubkey>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst).unwrap();
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, solana_program::program_error::ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!("Error: failed to deserialize account: {}", err);
            ProgramError::InvalidAccountData
        })
    }
}
