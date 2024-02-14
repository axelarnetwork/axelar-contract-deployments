//! Module for the GatewayApprovedMessage account type.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::keccak::hashv;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::accounts::discriminator::{Discriminator, MessageID};
use crate::types::execute_data_decoder::{DecodedCommand, DecodedMessage};

/// Possible statuses for a [`GatewayApprovedMessage`].
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
pub enum MessageApprovalStatus {
    /// Message is still awaiting to be approved.
    Pending,
    /// Message was approved
    Approved,
}

/// Gateway Approved Message type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayApprovedMessage {
    discriminator: Discriminator<MessageID>,
    status: MessageApprovalStatus,
}

impl GatewayApprovedMessage {
    /// Returns a message with pending approval.
    pub const fn pending() -> Self {
        Self {
            discriminator: Discriminator::new(),
            status: MessageApprovalStatus::Pending,
        }
    }

    /// Returns an approved message.
    pub const fn approved() -> Self {
        Self {
            discriminator: Discriminator::new(),
            status: MessageApprovalStatus::Approved,
        }
    }

    /// Returns `true` if this message was approved.
    pub fn is_approved(&self) -> bool {
        matches!(self.status, MessageApprovalStatus::Approved)
    }

    /// Returns `true` if this message is still waiting for aproval.
    pub fn is_pending(&self) -> bool {
        matches!(self.status, MessageApprovalStatus::Pending)
    }

    /// Finds a PDA for this account by hashing the parameters. Returns its
    /// Pubkey and bump.
    ///
    ///`source_chain` and `source_address` are expected as byte-slices, leaving
    /// the conversions to the caller's discretion.
    pub fn pda(
        message_id: [u8; 32],
        source_chain: &[u8],
        source_address: &[u8],
        payload_hash: [u8; 32],
    ) -> (Pubkey, u8) {
        let (pubkey, bump, _seed) =
            Self::pda_with_seed(message_id, source_chain, source_address, payload_hash);
        (pubkey, bump)
    }

    /// Finds a PDA for this account by hashing the parameters. Returns its
    /// Pubkey, the bump and the seed used to derive it.
    ///
    ///`source_chain` and `source_address` are expected as byte-slices, leaving
    /// the conversions to the caller's discretion.
    pub fn pda_with_seed(
        message_id: [u8; 32],
        source_chain: &[u8],
        source_address: &[u8],
        payload_hash: [u8; 32],
    ) -> (Pubkey, u8, [u8; 32]) {
        let seeds: &[&[u8]] = &[&message_id, source_chain, source_address, &payload_hash];
        // Hashing is necessary because seed elements have arbitrary size.
        let seeds_hash = hashv(seeds).to_bytes();
        let (pda, bump) = Pubkey::find_program_address(&[seeds_hash.as_slice()], &crate::ID);
        (pda, bump, seeds_hash)
    }

    /// Finds the PDA for an Approved Message account from a `DecodedCommand`
    pub fn pda_from_decoded_command(command: &DecodedCommand) -> Pubkey {
        let DecodedMessage {
            id,
            source_chain,
            source_address,
            payload_hash,
            ..
        } = &command.message;
        let (pda, _bump) = Self::pda(
            *id,
            source_chain.as_bytes(),
            source_address.as_bytes(),
            *payload_hash,
        );
        pda
    }
}

impl Sealed for GatewayApprovedMessage {}

impl Pack for GatewayApprovedMessage {
    const LEN: usize = 9;

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
