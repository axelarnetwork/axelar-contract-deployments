//! Event discriminators (prefixes) used to identify logged events related to native gas operations.

/// Prefix emitted when native gas is paid for a contract call.
pub const NATIVE_GAS_PAID_FOR_CONTRACT_CALL: &[u8] = b"native gas paid for contract call";
/// Prefix emitted when native gas is added to an already emtted contract call.
pub const NATIVE_GAS_ADDED: &[u8] = b"native gas added";
/// Prefix emitted when native gas is refunded.
pub const NATIVE_GAS_REFUNDED: &[u8] = b"native gas refunded";

/// Prefix emitted when SPL token was used to pay for a contract call.
pub const SPL_PAID_FOR_CONTRACT_CALL: &[u8] = b"spl token paid for contract call";
/// Prefix emitted when SPL token gas is added to an already emtted contract call.
pub const SPL_GAS_ADDED: &[u8] = b"spl token gas added";
/// Prefix emitted when SPL token gas is refunded.
pub const SPL_GAS_REFUNDED: &[u8] = b"spl token refunded";
