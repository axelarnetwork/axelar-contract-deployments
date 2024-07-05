#![allow(missing_docs)]

use std::io::prelude::{Read, Write};
use std::io::{self};

use axelar_message_primitives::DestinationProgramId;
use axelar_rkyv_encoding::types::{ArchivedMessage, ArchivedVerifierSet, Message, VerifierSet};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

pub enum CommandKind {
    ApproveMessage,
    RotateSigner,
}

pub trait Command {
    fn kind(&self) -> CommandKind;
    fn axelar_message(&self) -> Option<&impl AxelarMessage>;
    fn hash(&self) -> [u8; 32];
}

impl Command for OwnedCommand {
    #[inline]
    fn kind(&self) -> CommandKind {
        match self {
            OwnedCommand::ApproveMessage(_) => CommandKind::ApproveMessage,
            OwnedCommand::RotateSigners(_) => CommandKind::RotateSigner,
        }
    }

    #[inline]
    fn axelar_message(&self) -> Option<&impl AxelarMessage> {
        match self {
            OwnedCommand::ApproveMessage(message) => Some(message),
            OwnedCommand::RotateSigners(_) => None,
        }
    }

    #[inline]
    fn hash(&self) -> [u8; 32] {
        match self {
            OwnedCommand::ApproveMessage(message) => message.hash(),
            OwnedCommand::RotateSigners(verifier_set) => verifier_set.hash(),
        }
    }
}

impl Command for ArchivedCommand<'_> {
    fn kind(&self) -> CommandKind {
        match self {
            ArchivedCommand::ApproveMessage(_) => CommandKind::ApproveMessage,
            ArchivedCommand::RotateSigners(_) => CommandKind::RotateSigner,
        }
    }

    fn axelar_message(&self) -> Option<&impl AxelarMessage> {
        match self {
            ArchivedCommand::ApproveMessage(message) => Some(message),
            ArchivedCommand::RotateSigners(_) => None,
        }
    }

    fn hash(&self) -> [u8; 32] {
        match self {
            ArchivedCommand::ApproveMessage(message) => message.hash(),
            ArchivedCommand::RotateSigners(verifier_set) => verifier_set.hash(),
        }
    }
}

pub trait AxelarMessage {
    fn hash(&self) -> [u8; 32];

    fn destination_program(&self) -> Result<DestinationProgramId, GatewayError>;
}

impl AxelarMessage for &ArchivedMessage {
    fn hash(&self) -> [u8; 32] {
        ArchivedMessage::hash(self)
    }

    fn destination_program(&self) -> Result<DestinationProgramId, GatewayError> {
        let pubkey: Pubkey = self
            .destination_address()
            .parse()
            .map_err(|_| GatewayError::PublicKeyParseError)?;
        Ok(pubkey.into())
    }
}

impl AxelarMessage for Message {
    fn hash(&self) -> [u8; 32] {
        Message::hash(self)
    }

    fn destination_program(&self) -> Result<DestinationProgramId, GatewayError> {
        let pubkey: Pubkey = self
            .destination_address()
            .parse()
            .map_err(|_| GatewayError::PublicKeyParseError)?;
        Ok(pubkey.into())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ArchivedCommand<'a> {
    ApproveMessage(&'a ArchivedMessage),
    RotateSigners(&'a ArchivedVerifierSet),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnedCommand {
    ApproveMessage(Message),
    RotateSigners(VerifierSet),
}

impl<'a> From<&'a ArchivedMessage> for ArchivedCommand<'a> {
    fn from(message: &'a ArchivedMessage) -> Self {
        ArchivedCommand::ApproveMessage(message)
    }
}

impl<'a> From<&'a ArchivedVerifierSet> for ArchivedCommand<'a> {
    fn from(verifier_set: &'a ArchivedVerifierSet) -> Self {
        ArchivedCommand::RotateSigners(verifier_set)
    }
}

impl BorshSerialize for OwnedCommand {
    fn serialize<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        match self {
            OwnedCommand::ApproveMessage(message) => {
                0u8.serialize(writer)?; //  discriminant
                message.to_bytes()
            }
            OwnedCommand::RotateSigners(verifier_set) => {
                1u8.serialize(writer)?; // discriminant
                verifier_set.to_bytes()
            }
        }
        .map_err(io::Error::other)
        .and_then(|bytes| bytes.serialize(writer))
    }
}

impl BorshDeserialize for OwnedCommand {
    fn deserialize_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let discriminant = u8::deserialize_reader(reader)?;
        let bytes = Vec::<u8>::deserialize_reader(reader)?;

        let command = match discriminant {
            0 => {
                OwnedCommand::ApproveMessage(Message::from_bytes(&bytes).map_err(io::Error::other)?)
            }
            1 => OwnedCommand::RotateSigners(
                VerifierSet::from_bytes(&bytes).map_err(io::Error::other)?,
            ),
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid command discriminant byte: {other}"),
                ))
            }
        };
        Ok(command)
    }
}

/// FIXME: This is a workaround to wrap the serialized Message from the
/// `axelar-rkyv-encoding` crate. It shoud not be needed once we fully migrate
/// types from that crate.
#[derive(BorshDeserialize, BorshSerialize, PartialEq, Eq, Debug, Clone)]
pub struct MessageWrapper {
    pub serialized_message: Vec<u8>,
}

impl TryFrom<Message> for MessageWrapper {
    type Error = ProgramError;

    fn try_from(message: Message) -> Result<Self, Self::Error> {
        let serialized_message = message
            .to_bytes()
            .map_err(|_| ProgramError::BorshIoError("failed to serialize Message".into()))?;
        Ok(Self { serialized_message })
    }
}

impl<'a> TryFrom<&'a MessageWrapper> for &'a ArchivedMessage {
    type Error = ProgramError;

    fn try_from(wrapper: &'a MessageWrapper) -> Result<Self, Self::Error> {
        let MessageWrapper { serialized_message } = wrapper;
        ArchivedMessage::from_archived_bytes(serialized_message).map_err(|err| {
            solana_program::msg!("decode err {:?}", err);
            ProgramError::BorshIoError("failed to serialize Message".into())
        })
    }
}

#[test]
fn message_wrapper_roundtrip() {
    use axelar_rkyv_encoding::test_fixtures::random_message;
    let original = random_message();

    let wrapped: MessageWrapper = original.clone().try_into().unwrap();

    let unwrapped = Message::from_bytes(&wrapped.serialized_message).unwrap();
    let archived: &ArchivedMessage = (&wrapped).try_into().unwrap();

    assert_eq!(original, unwrapped);
    assert_eq!(original.hash(), unwrapped.hash());

    assert_eq!(&original, archived);
    assert_eq!(original.hash(), archived.hash());
}
