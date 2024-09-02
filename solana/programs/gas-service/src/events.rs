//! Axelar Gas Service events.

use axelar_message_primitives::U256;
use base64::engine::general_purpose;
use base64::Engine as _;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::keccak::hash;
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::{LogIndex, TxHash};

/// Gas Service program logs.
#[non_exhaustive]
#[repr(u8)]
#[derive(Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub enum GasServiceEvent {
    /// Logged when the Gas Service receives an payment for contract call in
    /// native currency.
    NativeGasPaidForContractCall {
        /// [sender] The address making the payment.
        sender: Pubkey,

        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload_hash ] Data payload hash.
        payload_hash: [u8; 32],

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service receives an payment for contract call in
    /// native currency with token.
    NativeGasPaidForContractCallWithToken {
        /// [sender] The address making the payment.
        sender: Pubkey,

        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload_hash ] Data payload hash.
        payload_hash: [u8; 32],

        /// [symbol] The symbol of the token used to pay for gas.
        symbol: Vec<u8>,

        /// [amount] The amount of tokens to pay for gas.
        amount: U256,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service receives an payment for express call.
    NativeGasPaidForExpressCall {
        /// [sender] The address making the payment.
        sender: Pubkey,

        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload_hash ] Data payload hash.
        payload_hash: [u8; 32],

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service receives an payment for express call with
    /// token.
    NativeGasPaidForExpressCallWithToken {
        /// [sender] The address making the payment.
        sender: Pubkey,

        /// [destination_chain] The target chain.
        destination_chain: Vec<u8>,

        /// [destination_address] The target address on the destination chain.
        destination_address: Vec<u8>,

        /// [payload_hash ] Data payload hash.
        payload_hash: [u8; 32],

        /// [symbol] The symbol of the token to be sent with the call.
        symbol: Vec<u8>,

        /// [amount] The amount of tokens to be sent with the call.
        amount: U256,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service receives an payment for gas in native
    /// currency.
    NativeGasAdded {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service receives an payment for gas with native
    /// currency.
    NativeExpressGasAdded {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },

    /// Logged when the Gas Service refunds fees, if any, in native currency.
    Refunded {
        /// [tx_hash] The transaction hash of the cross-chain call.
        tx_hash: TxHash,

        /// [log_index] The log index of the event.
        log_index: LogIndex,

        /// [fees] The amount of SOL to pay for gas.
        fees: u64,

        /// [refund_address] The address where refunds, if any, should be sent.
        refund_address: Pubkey,
    },
}

impl GasServiceEvent {
    /// Emits the log for this event.
    pub fn emit(&self) -> Result<(), ProgramError> {
        let serialized = borsh::to_vec(self)?;
        sol_log_data(&[&serialized]);
        Ok(())
    }

    /// Try to parse a [`GasServiceEvent`] out of a Solana program log line.
    pub fn parse_log<T: AsRef<str>>(log: T) -> Option<Self> {
        let cleaned_input = log
            .as_ref()
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;
        borsh::from_slice(&cleaned_input).ok()
    }
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}

/// Emit a [`NativeGasPaidForContractCall`].
pub fn emit_native_gas_paid_for_contract_call_event(
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeGasPaidForContractCall {
        sender,
        destination_chain,
        destination_address,
        payload_hash: hash(&payload).to_bytes(),
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`NativeGasPaidForContractCallWithToken`].
#[allow(clippy::too_many_arguments)]
pub fn emit_native_gas_paid_for_contract_call_with_token_event(
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    symbol: Vec<u8>,
    amount: U256,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeGasPaidForContractCallWithToken {
        sender,
        destination_chain,
        destination_address,
        payload_hash: hash(&payload).to_bytes(),
        symbol,
        amount,
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`NativeGasPaidForExpressCall`].
pub fn emit_native_gas_paid_for_express_call_event(
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeGasPaidForExpressCall {
        sender,
        destination_chain,
        destination_address,
        payload_hash: hash(&payload).to_bytes(),
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`NativeGasPaidForExpressCallWithToken`].
#[allow(clippy::too_many_arguments)]
pub fn emit_native_gas_paid_for_express_call_with_token_event(
    sender: Pubkey,
    destination_chain: Vec<u8>,
    destination_address: Vec<u8>,
    payload: Vec<u8>,
    symbol: Vec<u8>,
    amount: U256,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeGasPaidForExpressCallWithToken {
        sender,
        destination_chain,
        destination_address,
        payload_hash: hash(&payload).to_bytes(),
        symbol,
        amount,
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`NativeGasAdded`].
pub fn emit_native_gas_added_event(
    tx_hash: TxHash,
    log_index: LogIndex,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeGasAdded {
        tx_hash,
        log_index,
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`NativeExpressGasAdded`].
pub fn emit_native_express_gas_added_event(
    tx_hash: TxHash,
    log_index: LogIndex,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::NativeExpressGasAdded {
        tx_hash,
        log_index,
        fees,
        refund_address,
    };
    event.emit()
}

/// Emit a [`Refunded`].
pub fn emit_refunded_event(
    tx_hash: TxHash,
    log_index: LogIndex,
    fees: u64,
    refund_address: Pubkey,
) -> Result<(), ProgramError> {
    let event = GasServiceEvent::Refunded {
        tx_hash,
        log_index,
        fees,
        refund_address,
    };
    event.emit()
}
