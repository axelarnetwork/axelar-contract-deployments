//! Governance program logs.
///
/// Following the [implementation](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/main/contracts/interfaces/IInterchainGovernance.sol#L20-L40)
use anchor_discriminators::Discriminator;
use event_cpi_macros::event;

/// Logged when the governance program receives and successfully processes
/// an incoming Axelar governance gmp message from the Axelar network.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProposalScheduled {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
    /// Unix timestamp in seconds from when the proposal can be executed.
    pub eta: [u8; 32],
}

/// Logged when the governance program receives and successfully processes
/// an incoming Axelar governance gmp proposal cancellation message from the
/// Axelar network.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProposalCancelled {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
    /// Unix timestamp in seconds from when the proposal can be executed
    /// little-endian U64 value. Limbs are in little-endian order.
    pub eta: [u8; 32],
}
/// Logged when a previously scheduled proposal is executed.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProposalExecuted {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
    /// Unix timestamp in seconds from when the proposal can be executed
    /// little-endian U64 value. Limbs are in little-endian order.
    pub eta: [u8; 32],
}

/// Logged when the Axelar governance infrastructure marks a proposal as
/// directly executable by the operator.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorProposalApproved {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
}

/// Logged when the Axelar governance infrastructure marks a proposal as
/// non directly executable by the operator.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorProposalCancelled {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
}

/// Logged when the operator executes a proposal that was previously put
/// under it's approval by the Axelar governance infrastructure.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorProposalExecuted {
    /// The hash of the proposal in which the PDA derivation was based.
    /// The hash is crafted from the target address, call data and native
    /// value.
    pub hash: [u8; 32],
    /// The target address represented as a 32-byte array. It represents the
    /// [`solana_program::pubkey::Pubkey`].
    pub target_address: [u8; 32],
    /// The call data required to execute the target program.
    /// See [`crate::proposal::ExecuteProposalCallData`].
    pub call_data: Vec<u8>,
    /// This field represents how many native tokens (lamports) are being
    /// sent to the target program. It's a little-endian U256 value.
    pub native_value: [u8; 32],
}

/// Logged when the operator transfers it's operatorship to another account.
#[event]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorshipTransferred {
    /// The previous operator account.
    pub old_operator: [u8; 32],
    /// The new operator account.
    pub new_operator: [u8; 32],
}
