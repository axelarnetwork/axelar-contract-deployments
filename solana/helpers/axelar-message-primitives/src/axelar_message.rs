use std::borrow::Cow;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::payload::DataPayloadHash;

/// Represents different unique parameters for a given Axelar message
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AxelarMessageParams<'a> {
    /// The message id
    pub command_id: CommandId<'a>,
    /// The source chain
    pub source_chain: SourceChain<'a>,
    /// The source address
    pub source_address: SourceAddress<'a>,
    /// The destination pubkey
    pub destination_program: DestinationProgramId,
    /// The payload hash
    pub payload_hash: DataPayloadHash<'a>,
}

impl<'a> From<&'a connection_router::Message> for AxelarMessageParams<'a> {
    fn from(message: &'a connection_router::Message) -> Self {
        let message_id =
            solana_program::keccak::hash(message.cc_id.to_string().as_bytes()).to_bytes();
        let source_chain = message.cc_id.chain.to_string();

        let message_id = CommandId(Cow::Owned(message_id));
        let source_chain = SourceChain(Cow::Owned(source_chain));
        let source_address = SourceAddress(message.source_address.as_bytes());

        // Currently the hex encoding is enforced by the bcs encoding when it processes
        // the message But we should consider enforcing it here as well.
        // TODO: switch to TryFrom
        let mut destination_pubkey = [0; 32];
        hex::decode_to_slice(
            message.destination_address.as_bytes(),
            &mut destination_pubkey,
        )
        .expect("Failed to parse source address");
        let destination_pubkey = DestinationProgramId(Pubkey::new_from_array(destination_pubkey));
        let payload_hash = DataPayloadHash(Cow::Borrowed(&message.payload_hash));
        AxelarMessageParams {
            command_id: message_id,
            source_chain,
            source_address,
            destination_program: destination_pubkey,
            payload_hash,
        }
    }
}

/// Newtype for a message ID.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommandId<'a>(pub Cow<'a, [u8; 32]>);

/// Newtype for a source chain.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceChain<'a>(pub Cow<'a, String>);

/// Newtype for a source address.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceAddress<'a>(pub &'a [u8]);

/// Newtype for a destination address.
/// This is the program ID of the destination program.
#[derive(Debug, PartialEq, Eq, Copy, Clone, BorshSerialize, BorshDeserialize)]
pub struct DestinationProgramId(pub Pubkey);

impl From<Pubkey> for DestinationProgramId {
    fn from(pubkey: Pubkey) -> Self {
        DestinationProgramId(pubkey)
    }
}

impl DestinationProgramId {
    /// Returns the signing PDA for this destination address and message ID.
    ///
    /// Only the destination program is allowed to sign the message for
    /// validating that a message is being executed - this is reference to
    /// gateway.validateContractCall.
    pub fn signing_pda(&self, command_id: &CommandId) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[command_id.0.as_ref()], &self.0)
    }
}
