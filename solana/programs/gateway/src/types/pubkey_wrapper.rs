//! Module for the [`PubkeyWrapper`] type.

use solana_program::pubkey::Pubkey;

/// Wrapper type used to implement Borsh traits for [`Pubkey`]
// #[repr(transparent)]
// #[derive(Clone, Debug, PartialEq, Eq)]
pub type PubkeyWrapper = Pubkey;
