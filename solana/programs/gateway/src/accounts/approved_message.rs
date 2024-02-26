//! Module for the GatewayApprovedMessage account type.

use std::borrow::Cow;
use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;

use crate::accounts::discriminator::{Discriminator, MessageID};
use crate::error::GatewayError;
use crate::types::execute_data_decoder::{DecodedCommand, DecodedMessage};

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
    pub allowed_executer: Pubkey,
}

impl GatewayApprovedMessage {
    /// Returns an approved message.
    pub fn pending(allowed_executer: Pubkey) -> Self {
        Self {
            discriminator: Discriminator::new(),
            status: MessageApprovalStatus::Pending,
            allowed_executer,
        }
    }

    /// Makes sure that the attached account info is the expected one
    /// If successful verification: will update the status to `Executed`
    pub fn verify_caller(&mut self, caller: &AccountInfo<'_>) -> Result<(), ProgramError> {
        if !caller.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if caller.key != &self.allowed_executer {
            return Err(GatewayError::InvalidExecutor.into());
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
        message_id: &MessageId,
        source_chain: &SourceChain,
        source_address: &SourceAddress,
        payload_hash: &PayloadHash,
    ) -> (Pubkey, u8, [u8; 32]) {
        use solana_program::keccak::hashv;

        // TODO we should replace the gateway root pda with the execute data pda.
        //      It intrinsicly would depend on the gateway pda + would link with the
        // expected execute data acc
        let seeds: &[&[u8]] = &[
            gateway_root_pda.as_ref(),
            message_id.0.as_ref(),
            source_chain.0.as_bytes(),
            source_address.0,
            payload_hash.0,
        ];

        // Hashing is necessary because otherwise the seeds would be too long
        let seeds_hash = hashv(seeds).to_bytes();

        let (pubkey, bump) = Pubkey::find_program_address(&[seeds_hash.as_ref()], &crate::ID);
        (pubkey, bump, seeds_hash)
    }

    /// Finds a PDA for this account from a given Axelar source message
    pub fn pda_from_axelar_message(
        gateway_root_pda: Pubkey,
        message: &connection_router::Message,
    ) -> (
        Pubkey,
        u8,
        [u8; 32],
        MessageId,
        SourceChain,
        SourceAddress,
        PayloadHash,
    ) {
        let message_id =
            solana_program::keccak::hash(message.cc_id.to_string().as_bytes()).to_bytes();
        let source_chain = message.cc_id.chain.to_string();

        let message_id = MessageId(Cow::Owned(message_id));
        let source_chain = SourceChain(Cow::Owned(source_chain));
        let source_address = SourceAddress(message.source_address.as_bytes());
        let payload_hash = PayloadHash(&message.payload_hash);

        let (pda, bump, seed) = Self::pda(
            &gateway_root_pda,
            &message_id,
            &source_chain,
            &source_address,
            &payload_hash,
        );

        (
            pda,
            bump,
            seed,
            message_id,
            source_chain,
            source_address,
            payload_hash,
        )
    }

    /// Finds the PDA for an Approved Message account from a `DecodedCommand`
    pub fn pda_from_decoded_command(gateway_root_pda: Pubkey, command: &DecodedCommand) -> Pubkey {
        let DecodedMessage {
            id,
            source_chain,
            source_address,
            payload_hash,
            ..
        } = &command.message;

        let message_id = MessageId(Cow::Borrowed(id));
        let source_chain = SourceChain(Cow::Borrowed(source_chain));
        let source_address = SourceAddress(source_address.as_bytes());
        let payload_hash = PayloadHash(payload_hash);

        let (pda, _bump, _seed) = Self::pda(
            &gateway_root_pda,
            &message_id,
            &source_chain,
            &source_address,
            &payload_hash,
        );
        pda
    }
}

/// Newtype for a message ID.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MessageId<'a>(pub Cow<'a, [u8; 32]>);

/// Newtype for a source chain.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceChain<'a>(pub Cow<'a, String>);

/// Newtype for a source address.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceAddress<'a>(pub &'a [u8]);

/// Newtype for a payload hash.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PayloadHash<'a>(pub &'a [u8; 32]);

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
