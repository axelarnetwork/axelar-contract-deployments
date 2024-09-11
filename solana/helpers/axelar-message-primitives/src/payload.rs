//! This module contains the `DataPayload` struct, which represents a payload
//! in the standard Axelar flow.

use std::borrow::Cow;
use std::ops::Deref;

use alloy_sol_types::sol;
use solana_program::instruction::AccountMeta;
use solana_program::program_error::ProgramError;

pub use self::encoding::EncodingScheme;

mod encoding;

/// Newtype for a payload hash.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataPayloadHash<'a>(pub Cow<'a, [u8; 32]>);

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
pub struct DataPayload<'payload> {
    // Using Cow because on-chain we will use a the owned version (because of the decoding),
    // but off-chain we will use the borrowed version to prevent unnecessary cloning.
    payload_without_accounts: Cow<'payload, [u8]>,
    solana_accounts: Vec<SolanaAccountRepr>,
    encoding_scheme: EncodingScheme,
}

impl<'payload> DataPayload<'payload> {
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
        for acc in solana_accounts.iter() {
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
    pub fn new_with_cow(
        payload_without_accounts: Cow<'payload, [u8]>,
        solana_accounts: Vec<SolanaAccountRepr>,
        encoding_scheme: EncodingScheme,
    ) -> Self {
        Self {
            encoding_scheme,
            payload_without_accounts,
            solana_accounts,
        }
    }

    /// Get the payload hash.
    pub fn hash(&self) -> Result<DataPayloadHash<'_>, PayloadError> {
        let payload = self.encode()?;
        let payload_hash = solana_program::keccak::hash(payload.as_slice()).to_bytes();

        Ok(DataPayloadHash(Cow::Owned(payload_hash)))
    }

    /// Get the payload without accounts.
    pub fn payload_without_accounts(&self) -> &[u8] {
        self.payload_without_accounts.deref()
    }

    /// Get the solana accounts.
    pub fn account_meta(&self) -> &[AccountMeta] {
        // Safe cast because we know that the representation is correct
        unsafe { std::mem::transmute(self.solana_accounts.as_slice()) }
    }

    pub fn encoding_scheme(&self) -> EncodingScheme {
        self.encoding_scheme
    }
}

/// Error type for payload operations.
#[derive(Debug, thiserror::Error)]
pub enum PayloadError {
    #[error("Invalid encoding scheme")]
    InvalidEncodingScheme,
    #[error("Borsh serialize error")]
    BorshSerializeError,
    #[error("Borsh deserialize error")]
    BorshDeserializeError,
    #[error(transparent)]
    AbiError(#[from] alloy_sol_types::Error),
    #[error("Internal type conversion error")]
    Conversion,
}

impl From<PayloadError> for ProgramError {
    fn from(_value: PayloadError) -> Self {
        // TODO: Implement proper error conversion
        ProgramError::Custom(0)
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
    #[derive(Debug, PartialEq, Eq)]
    #[repr(C)]
    struct SolanaAccountRepr {
        bytes32 pubkey;
        bool is_signer;
        bool is_writable;
    }
}

// NOTE: Mostly used by tests
impl<'a> From<&'a SolanaAccountRepr> for SolanaAccountRepr {
    fn from(value: &'a SolanaAccountRepr) -> Self {
        value.clone()
    }
}

impl<'a, 'b> From<&'b solana_program::account_info::AccountInfo<'a>> for SolanaAccountRepr {
    fn from(account: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        SolanaAccountRepr {
            pubkey: account.key.to_bytes().into(),
            is_signer: account.is_signer,
            is_writable: account.is_writable,
        }
    }
}

impl<'a> From<&'a AccountMeta> for SolanaAccountRepr {
    fn from(value: &'a AccountMeta) -> Self {
        SolanaAccountRepr {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<AccountMeta> for SolanaAccountRepr {
    fn from(value: AccountMeta) -> Self {
        SolanaAccountRepr {
            pubkey: value.pubkey.to_bytes().into(),
            is_signer: value.is_signer,
            is_writable: value.is_writable,
        }
    }
}
impl From<SolanaAccountRepr> for AccountMeta {
    fn from(value: SolanaAccountRepr) -> Self {
        let pubkey_bytes: [u8; 32] = value.pubkey.into();

        AccountMeta {
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
