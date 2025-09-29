//! Events emitted by the Axelar Solana Gateway program.

use anchor_discriminators::Discriminator;
use axelar_message_primitives::U256;
use event_cpi_macros::event;
use solana_program::pubkey::Pubkey;

/// Event emitted when a contract call is initiated.
/// This event is emitted during the `call_contract` instruction.
/// - `sender_key`: 32-byte Solana public key
/// - `payload_hash`: 32-byte hash of the payload
/// - `destination_chain`: UTF-8 string identifying target blockchain
/// - `destination_contract_address`: UTF-8 string of target contract
/// - `payload`: Raw bytes to be transmitted cross-chain
#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallContractEvent {
    /// The sender's public key
    pub sender: Pubkey,
    /// Hash of the payload being sent
    pub payload_hash: [u8; 32],
    /// The destination chain identifier
    pub destination_chain: String,
    /// The destination contract address
    pub destination_contract_address: String,
    /// The raw payload data
    pub payload: Vec<u8>,
}

/// Event emitted when signers are rotated.
/// This event is emitted during the `rotate_signers` instruction.
#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierSetRotatedEvent {
    /// The epoch number as a 256-bit integer in little-endian format
    pub epoch: U256,
    /// Hash of the new verifier set
    pub verifier_set_hash: [u8; 32],
}

/// Event emitted when operatorship is transferred.
/// This event is emitted during the `transfer_operatorship` instruction.
#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorshipTransferredEvent {
    /// The new operator's public key
    pub new_operator: Pubkey,
}

/// Event emitted when a message is approved by the gateway.
/// This event is emitted during the `approve_message` instruction.
#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageApprovedEvent {
    /// The command ID for the message (32 bytes)
    pub command_id: [u8; 32],
    /// The destination address where the message will be delivered
    pub destination_address: Pubkey,
    /// Hash of the message payload
    pub payload_hash: [u8; 32],
    /// The source chain identifier
    pub source_chain: String,
    /// The command ID as string from the cross-chain ID
    pub cc_id: String,
    /// The source address that sent the message
    pub source_address: String,
    /// The destination chain identifier
    pub destination_chain: String,
}

/// Event emitted when a message is executed.
/// This event is emitted during the `validate_message` instruction.
#[event]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageExecutedEvent {
    /// The command ID for the message (32 bytes)
    pub command_id: [u8; 32],
    /// The destination address where the message was delivered
    pub destination_address: Pubkey,
    /// Hash of the message payload
    pub payload_hash: [u8; 32],
    /// The source chain identifier
    pub source_chain: String,
    /// The command ID as string from the cross-chain ID
    pub cc_id: String,
    /// The source address that sent the message
    pub source_address: String,
    /// The destination chain identifier
    pub destination_chain: String,
}

/// Represents the various events emitted by the Gateway.
///
/// The `GatewayEvent` enum encapsulates all possible events that can be emitted by the Gateway.
/// Each variant corresponds to a specific event type and contains the relevant data associated with that event.
///
/// These events are crucial for monitoring the state and actions within the Gateway, such as contract calls,
/// verifier set rotations, operatorship transfers, and message approvals and executions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayEvent {
    /// Represents a `CallContract` event.
    ///
    /// This event is emitted when a contract call is initiated to an external chain.
    CallContract(CallContractEvent),

    /// Represents a `VerifierSetRotatedEvent` event.
    VerifierSetRotated(VerifierSetRotatedEvent),

    /// Represents an `OperatorshipTransferred` event.
    ///
    /// This event is emitted when the operatorship is transferred to a new operator.
    /// It includes the public key of the new operator.
    OperatorshipTransferred(OperatorshipTransferredEvent),

    /// Represents a `MessageApproved` event.
    ///
    /// This event is emitted when a message is approved for execution by the Gateway.
    MessageApproved(MessageApprovedEvent),

    /// Represents a `MessageExecuted` event.
    ///
    /// This event is emitted when a message has been received & execution has begun on the destination contract.
    MessageExecuted(MessageExecutedEvent),
}

#[cfg(test)]
mod tests {
    use event_cpi::CpiEvent;

    use super::*;

    #[test]
    fn test_discriminator() {
        let event = CallContractEvent {
            sender: solana_program::pubkey::new_rand(),
            payload_hash: [0u8; 32],
            destination_chain: "Ethereum".to_string(),
            destination_contract_address: "0x1234567890abcdef".to_string(),
            payload: vec![1, 2, 3, 4],
        };

        println!(
            "CallContractEvent Discriminator: {:?}",
            CallContractEvent::DISCRIMINATOR
        );

        let data = event.data();
        #[allow(clippy::indexing_slicing)]
        let data = &data[..8];
        assert_eq!(data, CallContractEvent::DISCRIMINATOR);
    }
}
