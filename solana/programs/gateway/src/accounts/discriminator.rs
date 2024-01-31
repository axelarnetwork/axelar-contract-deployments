//! Module for the `Discriminator` type.

use std::io;
use std::marker::PhantomData;
use std::str::{from_utf8, Utf8Error};

use borsh::{BorshDeserialize, BorshSerialize};

/// Trait for specifying discriminator values
pub trait DiscriminatorTrait {
    /// The discriminator UTF-8 bytes.
    const DISCRIMINATOR: &'static [u8; 8];

    /// The discriminator bytes represented as a [`&str`].
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
    /// Returns a new discriminator.
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

/// `TransferOperatorship` discriminator type
#[derive(BorshSerialize, Debug, PartialEq, Eq, Clone)]
pub struct TransferOperatorship;

impl DiscriminatorTrait for TransferOperatorship {
    const DISCRIMINATOR: &'static [u8; 8] = b"GwTrnOps";
}
