//! Structs for Gateway program accounts.

use auth_weighted::types::proof::Proof;
use borsh::{BorshDeserialize, BorshSerialize};
use discriminators::{Config, Discriminator, ExecuteData};
use solana_program::hash::hash;
use solana_program::keccak::hashv;
use solana_program::pubkey::Pubkey;

use self::discriminators::MessageID;
use crate::error::GatewayError;
use crate::types::execute_data_decoder::{
    decode as decode_execute_data, DecodedCommand, DecodedCommandBatch, DecodedMessage,
};

/// Gateway configuration type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayConfig {
    /// TODO: Change this data type to include the Operators sets
    discriminator: Discriminator<Config>,
    version: u8,
}

impl GatewayConfig {
    /// Creates a new
    pub fn new(version: u8) -> Self {
        Self {
            discriminator: Discriminator::new(),
            version,
        }
    }

    /// Returns the Pubkey and canonical bump for this account.
    pub fn pda() -> (Pubkey, u8) {
        crate::find_root_pda()
    }
}

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

mod discriminators {
    use std::io;
    use std::marker::PhantomData;
    use std::str::{from_utf8, Utf8Error};

    use borsh::{BorshDeserialize, BorshSerialize};

    /// Trait for specifying discriminator values
    pub trait DiscriminatorTrait {
        const DISCRIMINATOR: &'static [u8; 8];
        fn discriminator() -> Result<&'static str, Utf8Error> {
            from_utf8(Self::DISCRIMINATOR)
        }
    }

    /// Generic discriminator wrapper type.
    #[derive(Debug, PartialEq, Eq, Clone)]
    #[repr(transparent)]
    pub struct Discriminator<T: DiscriminatorTrait> {
        _marker: PhantomData<T>,
    }

    impl<T: DiscriminatorTrait> Discriminator<T> {
        pub const fn new() -> Self {
            Self {
                _marker: PhantomData,
            }
        }
    }

    impl<T: BorshSerialize> BorshSerialize for Discriminator<T>
    where
        T: DiscriminatorTrait,
    {
        fn serialize<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
            writer.write_all(T::DISCRIMINATOR)
        }
    }

    impl<T: DiscriminatorTrait> BorshDeserialize for Discriminator<T> {
        fn deserialize_reader<R: io::Read>(reader: &mut R) -> io::Result<Self> {
            let discriminator: [u8; 8] = <_>::deserialize_reader(reader)?;

            if &discriminator != T::DISCRIMINATOR {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid discriminator",
                ));
            }
            Ok(Self {
                _marker: PhantomData,
            })
        }
    }

    // ----------------------------------------------------------------------
    // Concrete discriminators
    // ----------------------------------------------------------------------

    /// `GatewayConfig` discriminator type
    #[derive(BorshSerialize, Debug, PartialEq, Eq, Clone)]
    pub struct Config;

    impl DiscriminatorTrait for Config {
        const DISCRIMINATOR: &'static [u8; 8] = b"GwConfig";
    }

    /// `GatewayExecuteMessage` discriminator type
    #[derive(BorshSerialize, Debug, PartialEq, Eq, Clone)]
    pub struct ExecuteData;

    impl DiscriminatorTrait for ExecuteData {
        const DISCRIMINATOR: &'static [u8; 8] = b"GwExData";
    }

    /// `GatewayExecuteMessage` discriminator type
    #[derive(BorshSerialize, Debug, PartialEq, Eq, Clone)]
    pub struct MessageID;

    impl DiscriminatorTrait for MessageID {
        const DISCRIMINATOR: &'static [u8; 8] = b"GwMsgId1";
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;
    use crate::accounts::discriminators::DiscriminatorTrait;

    #[test]
    fn config_deserialization() -> Result<()> {
        let mut bytes = vec![];
        bytes.extend_from_slice(discriminators::Config::DISCRIMINATOR);
        bytes.push(255); // Version
        let state = GatewayConfig::try_from_slice(&bytes)?;
        assert_eq!(state.version, 255);
        Ok(())
    }

    #[test]
    fn config_invalid_discriminator() -> Result<()> {
        let mut invalid_bytes = vec![];
        invalid_bytes.extend_from_slice(b"deadbeef"); // Invalid discriminator
        invalid_bytes.push(1); // Version

        let error = GatewayConfig::try_from_slice(&invalid_bytes)
            .unwrap_err()
            .into_inner()
            .unwrap()
            .to_string();
        assert_eq!(error, "Invalid discriminator");
        Ok(())
    }

    #[test]
    fn execute_data_deserialization() -> Result<()> {
        let mut bytes = vec![];
        bytes.extend_from_slice(discriminators::ExecuteData::DISCRIMINATOR);
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
