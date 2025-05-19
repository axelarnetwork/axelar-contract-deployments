//! Events emitted by the Governance program.

use base64::engine::general_purpose;
use base64::Engine;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use solana_program::log::sol_log_data;
use solana_program::program_error::ProgramError;

/// Governance program logs.
///
/// Following the [implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/interfaces/IInterchainGovernance.sol#L20-L40)
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum GovernanceEvent {
    /// Logged when the governance program receives and successfully processes
    /// an incoming Axelar governance gmp message from the Axelar network.
    ProposalScheduled {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
        /// Unix timestamp in seconds from when the proposal can be executed.
        eta: [u8; 32],
    },

    /// Logged when the governance program receives and successfully processes
    /// an incoming Axelar governance gmp proposal cancellation message from the
    /// Axelar network.
    ProposalCancelled {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
        /// Unix timestamp in seconds from when the proposal can be executed
        /// little-endian U64 value. Limbs are in little-endian order.
        eta: [u8; 32],
    },

    /// Logged when a previously scheduled proposal is executed.
    ProposalExecuted {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
        /// Unix timestamp in seconds from when the proposal can be executed
        /// little-endian U64 value. Limbs are in little-endian order.
        eta: [u8; 32],
    },

    /// Logged when the Axelar governance infrastructure marks a proposal as
    /// directly executable by the operator.
    OperatorProposalApproved {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
    },

    /// Logged when the Axelar governance infrastructure marks a proposal as
    /// non directly executable by the operator.
    OperatorProposalCancelled {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
    },

    /// Logged when the operator executes a proposal that was previously put
    /// under it's approval by the Axelar governance infrastructure.
    OperatorProposalExecuted {
        /// The hash of the proposal in which the PDA derivation was based.
        /// The hash is crafted from the target address, call data and native
        /// value.
        hash: [u8; 32],
        /// The target address represented as a 32-byte array. It represents the
        /// [`solana_program::pubkey::Pubkey`].
        target_address: [u8; 32],
        /// The call data required to execute the target program. The rkyv
        /// encoded [`crate::proposal::ExecuteProposalCallData`].
        call_data: Vec<u8>,
        /// This field represents how many native tokens (lamports) are being
        /// sent to the target program. It's a little-endian U256 value.
        native_value: [u8; 32],
    },

    /// Logged when the operator transfers it's operatorship to another account.
    OperatorshipTransferred {
        /// The previous operator account.
        old_operator: [u8; 32],
        /// The new operator account.
        new_operator: [u8; 32],
    },
}

// TODO: below types are getting repeated in the codebase, should be moved to a
// common place.

impl GovernanceEvent {
    /// Emit the encoded `GovernanceEvent` as a Solana program log.
    ///
    /// This method encodes the event and logs it using `sol_log_data`.
    ///
    /// # Errors
    ///
    /// Returns a `ProgramError` if the encoding fails.
    ///
    /// # Panics
    pub fn emit(&self) -> Result<(), ProgramError> {
        let item = self.encode();
        sol_log_data(&[&item]);
        Ok(())
    }

    /// Encode the [`GovernanceEvent`] into a [`Vec<u8>`] which satisfies rkyv
    /// alignment requirements
    ///
    /// # Panics
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        to_vec(self).expect("Able to encode the event")
    }

    /// Try to parse a [`GovernanceEvent`] out of a Solana program log line.
    #[allow(clippy::flat_map_option)]
    pub fn parse_log<T: AsRef<str>>(log: T) -> Option<EventContainer> {
        let buffer = log
            .as_ref()
            .trim()
            .trim_start_matches("Program data:")
            .split_whitespace()
            .flat_map(decode_base64)
            .next()?;

        EventContainer::new(buffer)
    }
}

/// Wrapper around the rkyv encoded [`GovernanceEvent`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventContainer {
    buffer: Vec<u8>,
}

impl EventContainer {
    /// Create a new [`EventContainer`] from an rkyv encoded [`GovernanceEvent`]
    ///
    /// The method will return `None` if the buffer cannod be deserialised into
    /// a valid [`ArchivedGovernanceEvent`]
    #[must_use]
    pub fn new(buffer: Vec<u8>) -> Option<Self> {
        // check if this is a valid event
        let _data = borsh::from_slice::<GovernanceEvent>(&buffer).ok()?;
        Some(Self { buffer })
    }

    /// Returns the parse of this [`EventContainer`].
    ///
    /// # Errors
    ///
    /// This function will return a parsing error if any.
    pub fn parse(&self) -> Result<GovernanceEvent, std::io::Error> {
        borsh::from_slice::<GovernanceEvent>(&self.buffer)
    }
}

#[inline]
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    general_purpose::STANDARD.decode(input).ok()
}
