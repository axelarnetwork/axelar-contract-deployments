//! Structs for Gateway program accounts.

use borsh::{BorshDeserialize, BorshSerialize};
use discriminators::{Config, Discriminator, ExecuteData};
use solana_program::hash::hash;
use solana_program::pubkey::Pubkey;

use self::discriminators::MessageID;

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
}

/// Gateway Message ID type.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
#[repr(C)]
pub struct GatewayMessageID {
    discriminator: Discriminator<MessageID>,
    message_id: String,
}

impl GatewayMessageID {
    /// Creates a new `GatewayMessageID` struct.
    pub fn new(message_id: String) -> Self {
        Self {
            discriminator: Discriminator::new(),
            message_id,
        }
    }
    /// Returns the seeds for this account PDA.
    pub fn seeds(&self) -> [u8; 32] {
        hash(self.message_id.as_bytes()).to_bytes()
    }

    /// Finds a PDA for this account. Returns its Pubkey, the canonical bump and
    /// the seeds used to derive them.
    pub fn pda(&self) -> (Pubkey, u8, [u8; 32]) {
        let seeds = self.seeds();
        let (pubkey, bump) = Pubkey::find_program_address(&[seeds.as_ref()], &crate::ID);
        (pubkey, bump, seeds)
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
        pub fn new() -> Self {
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

    #[test]
    fn message_id_pda() -> Result<()> {
        let message_id = GatewayMessageID::new("Hello!".to_string());
        let (expected_pda, bump_seed, seed) = message_id.pda();
        let actual_pda =
            Pubkey::create_program_address(&[seed.as_ref(), &[bump_seed]], &crate::ID)?;
        assert_eq!(expected_pda, actual_pda);
        Ok(())
    }
}
