//! Types used for logging messages.

use base64::engine::general_purpose;
use base64::Engine as _;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use crate::error::GatewayError;

/// Gateway program logs.
///
/// Used internally by the Gateway program to log messages.
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum GatewayEventRef<'a> {
    /// Logged when the Gateway receives an outbound message.
    CallContract {
        /// Message sender.
        sender: &'a Pubkey,
        /// The name of the target blockchain.
        destination_chain: &'a [u8],
        /// The address of the target contract in the destination blockchain.
        destination_address: &'a [u8],
        /// Contract call data.
        payload: &'a [u8],
        /// The payload hash.
        payload_hash: &'a [u8; 32],
    },
}

impl<'a> GatewayEventRef<'a> {
    /// Returns the event's discriminant byte.
    fn discriminant(&self) -> u8 {
        unsafe { *(self as *const Self as *const u8) }
    }
    /// Emits the log for this event.
    pub fn emit(&self) {
        match *self {
            GatewayEventRef::CallContract {
                sender,
                destination_chain,
                destination_address,
                payload_hash,
                payload,
            } => sol_log_data(&[
                &[self.discriminant()],
                sender.as_ref(),
                destination_chain,
                destination_address,
                payload,
                payload_hash,
            ]),
        };
    }
}

/// Owned version of [`GatewayEventRef`].
///
/// Used by tests and external crates that want to parse GatewayEvent log messages.
#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum GatewayEvent {
    /// Logged when the Gateway receives an outbound message.
    CallContract {
        /// Message sender.
        sender: Pubkey,
        /// The name of the target blockchain.
        destination_chain: Vec<u8>,
        /// The address of the target contract in the destination blockchain.
        destination_address: Vec<u8>,
        /// Contract call data.
        payload: Vec<u8>,
        /// The payload hash.
        payload_hash: [u8; 32],
    },
}

impl GatewayEvent {
    /// Try to parse a [`GatewayEvent`] out of a Solana program log line.
    pub fn parse_log(log: &str) -> Option<Self> {
        let mut iterator = log
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64);

        let tag: u8 = match iterator.next()?[..] {
            [tag] => tag,
            _ => return None,
        };

        match tag {
            0 => {
                let sender = iterator
                    .next()
                    .map(|bytes| Pubkey::try_from(bytes).ok())??;
                let destination_chain = iterator.next()?;
                let destination_address = iterator.next()?;
                let payload = iterator.next()?;
                let payload_hash = iterator.next()?.try_into().ok()?;
                Some(GatewayEvent::CallContract {
                    sender,
                    destination_chain,
                    destination_address,
                    payload,
                    payload_hash,
                })
            }
            _ => None,
        }
    }
}

/// Emits a [`ContractCallEventRef`].
pub fn emit_call_contract_event(
    sender: &Pubkey,
    destination_chain: &[u8],
    destination_contract_address: &[u8],
    payload: &[u8],
    payload_hash: &[u8; 32],
) -> Result<(), GatewayError> {
    let event = GatewayEventRef::CallContract {
        sender,
        destination_chain,
        destination_address: destination_contract_address,
        payload_hash,
        payload,
    };
    event.emit();
    Ok(())
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

#[test]
fn parse_solana_log_call_contract() -> Result<(), Box<dyn std::error::Error>> {
    let log = "Program data: AA== QWnzT2Qimh+VZZfyzl0d3qLhDpLF1PrGk3vJFQI43PM= ZXRoZXJldW0= L0Pd/1ZPsmDb14PVX8bkxwvhiGI= JNlsoXSvVvGhKcBcTO7L53LikorD1vMZpjjJANixrTg= TAC7ot/aHrh49v0fVL5WEx68Rz+LqMVK2zhpZMPDnqU=";

    let pubkey_bytes =
        hex::decode("4169f34f64229a1f956597f2ce5d1ddea2e10e92c5d4fac6937bc9150238dcf3")?;
    let sender = Pubkey::try_from(pubkey_bytes).unwrap();
    let destination_chain = "ethereum".as_bytes().to_vec();
    let destination_address = hex::decode("2F43DDFf564Fb260dbD783D55fc6E4c70Be18862")?;
    let payload = hex::decode("24d96ca174af56f1a129c05c4ceecbe772e2928ac3d6f319a638c900d8b1ad38")?;
    let payload_hash: [u8; 32] =
        hex::decode("4c00bba2dfda1eb878f6fd1f54be56131ebc473f8ba8c54adb386964c3c39ea5")?
            .try_into()
            .unwrap();

    let expected = GatewayEvent::CallContract {
        sender,
        destination_chain,
        destination_address,
        payload,
        payload_hash,
    };
    assert_eq!(GatewayEvent::parse_log(log), Some(expected));
    Ok(())
}
