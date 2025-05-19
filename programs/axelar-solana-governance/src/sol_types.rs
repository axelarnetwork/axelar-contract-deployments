//! This module contains all the types which
//! mimics the Solana types and are used as transport in programs
//! and instructions.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

/// This is the analogous type of [`solana_program::instruction::AccountMeta`]
/// It was created for easily serializing and deserializing the account metadata
/// with rkvy.
#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub struct SolanaAccountMetadata {
    /// The [`solana_program::pubkey::Pubkey`], converted to bytes.
    pub pubkey: [u8; 32],
    /// If this account is a signer of the transaction. See original
    /// [`solana_program::instruction::AccountMeta::is_signer`].
    pub is_signer: bool,
    /// If this account is writable. See original
    /// [`solana_program::instruction::AccountMeta::is_writable`].
    pub is_writable: bool,
}

impl From<&AccountMeta> for SolanaAccountMetadata {
    fn from(value: &AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

impl From<AccountMeta> for SolanaAccountMetadata {
    fn from(value: AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

impl From<&SolanaAccountMetadata> for AccountMeta {
    fn from(value: &SolanaAccountMetadata) -> Self {
        let pubkey = Pubkey::new_from_array(value.pubkey);
        if value.is_writable {
            Self::new(pubkey, value.is_signer)
        } else {
            Self::new_readonly(pubkey, value.is_signer)
        }
    }
}

impl From<SolanaAccountMetadata> for AccountMeta {
    fn from(value: SolanaAccountMetadata) -> Self {
        let pubkey = Pubkey::new_from_array(value.pubkey);
        if value.is_writable {
            Self::new(pubkey, value.is_signer)
        } else {
            Self::new_readonly(pubkey, value.is_signer)
        }
    }
}
