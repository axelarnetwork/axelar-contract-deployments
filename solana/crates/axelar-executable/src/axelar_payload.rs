//! This module contains the `AxelarMessagePayload` struct, which represents a
//! payload in the standard Axelar flow.

pub use self::encoding::EncodingScheme;
use alloy_sol_types::sol;
use core::ops::Deref;
use solana_program::account_info::AccountInfo;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;
use std::borrow::Cow;
use thiserror::Error;

mod encoding;

/// Newtype for a payload hash.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AxelarMessagePayloadHash<'a>(pub Cow<'a, [u8; 32]>);

impl<'a> Deref for AxelarMessagePayloadHash<'a> {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// In standard Axelar flow, the accounts are concatenated at the beginning of
/// the payload message. This struct represents a Solana account in a way that
/// can be easily serialized and deserialized.
///
/// The payload is encoded in the following way:
/// - the first byte is encoding scheme, encoded as an u8.
/// - the rest of the data is encoded([account array][payload bytes]). The
///   encoding depends on the encoding scheme.
///
/// ```text
/// [u8 scheme] encoded([account array][payload bytes])
/// ```
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct AxelarMessagePayload<'payload> {
    // Using Cow because on-chain we will use a the owned version (because of the decoding),
    // but off-chain we will use the borrowed version to prevent unnecessary cloning.
    payload_without_accounts: Cow<'payload, [u8]>,
    solana_accounts: Vec<SolanaAccountRepr>,
    encoding_scheme: EncodingScheme,
}

impl<'payload> AxelarMessagePayload<'payload> {
    /// Create a new payload from a "payload without accounts" and a list of
    /// accounts representations.
    pub fn new<T>(
        payload_without_accounts: &'payload [u8],
        solana_accounts: &[T],
        encoding_scheme: EncodingScheme,
    ) -> Self
    where
        for<'b> &'b T: Into<SolanaAccountRepr>,
    {
        let mut solana_accounts_parsed = Vec::with_capacity(solana_accounts.len());
        for acc in solana_accounts {
            solana_accounts_parsed.push(acc.into());
        }
        Self::new_with_cow(
            Cow::Borrowed(payload_without_accounts),
            solana_accounts_parsed,
            encoding_scheme,
        )
    }

    /// Create a new payload from a "payload without accounts" and a list of
    /// account representations.
    #[must_use]
    pub fn new_with_cow(
        payload_without_accounts: Cow<'payload, [u8]>,
        solana_accounts: Vec<SolanaAccountRepr>,
        encoding_scheme: EncodingScheme,
    ) -> Self {
        Self {
            payload_without_accounts,
            solana_accounts,
            encoding_scheme,
        }
    }

    /// Get the payload hash.
    ///
    /// # Errors
    /// - the payload struct cannot be encoded
    pub fn hash(&self) -> Result<AxelarMessagePayloadHash<'_>, PayloadError> {
        let payload = self.encode()?;
        let payload_hash = solana_program::keccak::hash(payload.as_slice()).to_bytes();

        Ok(AxelarMessagePayloadHash(Cow::Owned(payload_hash)))
    }

    /// Get the payload without accounts.
    #[must_use]
    pub fn payload_without_accounts(&self) -> &[u8] {
        &self.payload_without_accounts
    }

    /// Get the solana accounts.
    #[must_use]
    pub fn account_meta(&self) -> Vec<AccountMeta> {
        self.solana_accounts
            .iter()
            .copied()
            .map(Into::into)
            .collect()
    }

    /// Get an iterator over the Solana accounts
    pub fn solana_accounts(&self) -> impl Iterator<Item = &SolanaAccountRepr> {
        self.solana_accounts.iter()
    }

    /// Get the underlying encoding scheme used by the [`AxelarMessagePayload`]
    #[must_use]
    pub const fn encoding_scheme(&self) -> EncodingScheme {
        self.encoding_scheme
    }
}

/// Error type for payload operations.
#[derive(Debug, Error)]
pub enum PayloadError {
    /// Invalid encoding scheme
    #[error("Invalid encoding scheme")]
    InvalidEncodingScheme,

    /// Borsh serialization error
    #[error("Borsh serialize error")]
    BorshSerializeError,

    /// Borsh deserialization error
    #[error("Borsh deserialize error")]
    BorshDeserializeError,

    /// ABI error
    #[error(transparent)]
    AbiError(#[from] alloy_sol_types::Error),
}

impl From<PayloadError> for ProgramError {
    fn from(error: PayloadError) -> Self {
        match error {
            PayloadError::InvalidEncodingScheme => Self::Custom(100),
            PayloadError::BorshSerializeError => Self::Custom(101),
            PayloadError::BorshDeserializeError => Self::Custom(102),
            PayloadError::AbiError(_e) => Self::Custom(103),
        }
    }
}

sol! {
    /// Representation of a Solana account in a way that can be easily serialized
    /// for Payload consumption.
    ///
    /// This is the expected data type that will be used to represent Solana
    /// accounts in the serilaized payload format.
    ///
    /// Utility methods are provided to encode and decode the representation.
    #[derive(Debug, PartialEq, Eq, Copy)]
    #[repr(C)]
    struct SolanaAccountRepr {
        /// Solana Pubkey (decoded format -- raw bytes)
        bytes32 pubkey;
        /// flag to indicate if the account is signer
        bool is_signer;
        /// flag to indicate if the account is writable
        bool is_writable;
    }
}

impl PartialEq<AccountInfo<'_>> for SolanaAccountRepr {
    fn eq(&self, other: &AccountInfo<'_>) -> bool {
        self.pubkey.as_slice() == other.key.as_ref()
            && self.is_signer == other.is_signer
            && self.is_writable == other.is_writable
    }
}

// NOTE: Mostly used by tests
impl<'a> From<&'a Self> for SolanaAccountRepr {
    fn from(value: &'a Self) -> Self {
        *value
    }
}

impl<'a, 'b> From<&'b solana_program::account_info::AccountInfo<'a>> for SolanaAccountRepr {
    fn from(account: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        Self {
            pubkey: account.key.to_bytes().into(),
            is_signer: account.is_signer,
            is_writable: account.is_writable,
        }
    }
}

impl<'a> From<&'a AccountMeta> for SolanaAccountRepr {
    fn from(value: &'a AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<AccountMeta> for SolanaAccountRepr {
    fn from(value: AccountMeta) -> Self {
        Self {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<SolanaAccountRepr> for AccountMeta {
    fn from(value: SolanaAccountRepr) -> Self {
        let pubkey_bytes: [u8; 32] = value.pubkey.into();

        Self {
            pubkey: pubkey_bytes.into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solana_account_repr_account_info_conversions() {
        for (is_singer, is_writer) in &[(true, true), (true, false), (false, true), (false, false)]
        {
            let key = solana_program::pubkey::Pubkey::new_unique();
            let mut lamports = 100;
            let account = solana_program::account_info::AccountInfo::new(
                &key,
                *is_singer,
                *is_writer,
                &mut lamports,
                &mut [],
                &key,
                false,
                0,
            );
            let repr = SolanaAccountRepr::from(&account);
            assert_eq!(repr.is_signer, *is_singer, "Signer flag is gone!");
            assert_eq!(repr.is_writable, *is_writer, "Writable flag is gone!");
            assert_eq!(
                repr.pubkey.to_vec()[..],
                key.to_bytes()[..],
                "Pubkey does not match!"
            );
        }
    }
}
