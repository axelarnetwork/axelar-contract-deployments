//! This module contains the `DataPayload` struct, which represents a payload
//! in the standard Axelar flow.

use std::borrow::Cow;
use std::ops::Deref;

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

/// Newtype for a payload hash.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataPayloadHash<'a>(pub Cow<'a, [u8; 32]>);

/// In standard Axelar flow, the accounts are concatenated at the beginning of
/// the payload message. This struct represents a Solana account in a way that
/// can be easily serialized and deserialized.
#[derive(PartialEq, Debug, Eq, Clone)]
pub struct DataPayload<'payload> {
    // Using Cow because on-chain we will use a the owned version (because of the decoding),
    // but off-chain we will use the borrowed version to prevent unnecessary cloning.
    payload_without_accounts: Cow<'payload, [u8]>,
    solana_accounts: Vec<SolanaAccountRepr>,
}

impl<'payload> DataPayload<'payload> {
    /// Create a new payload from a "payload without accounts" and a list of
    /// accounts representations.
    pub fn new<T>(payload_without_accounts: &'payload [u8], solana_accounts: &[T]) -> Self
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
        )
    }

    /// Create a new payload from a "payload without accounts" and a list of
    /// account representations.
    pub fn new_with_cow(
        payload_without_accounts: Cow<'payload, [u8]>,
        solana_accounts: Vec<SolanaAccountRepr>,
    ) -> Self {
        Self {
            payload_without_accounts,
            solana_accounts,
        }
    }

    /// Get the payload hash.
    pub fn hash(&self) -> DataPayloadHash<'_> {
        let payload = self.encode();
        let payload_hash = solana_program::keccak::hash(payload.as_slice()).to_bytes();

        DataPayloadHash(Cow::Owned(payload_hash))
    }

    /// Encode the payload
    ///
    /// Currently we are using Borsh for serialization and deserialization.
    pub fn encode(&self) -> Vec<u8> {
        // TODO:
        // 1. Eliminate the clone via `.to_vec()` but then it's hard to deserialize
        // 2. We need to switch AWAY from Borsh to something that can be easily digested
        //    by EVM and other chains
        borsh::to_vec::<(Vec<u8>, Vec<SolanaAccountRepr>)>(&(
            self.payload_without_accounts.to_vec(),
            self.solana_accounts.to_vec(),
        ))
        .unwrap()
    }

    /// Decode the payload from byte slice
    ///
    /// Currently we are using Borsh for serialization and deserialization.
    pub fn decode(data: &'payload [u8]) -> Self {
        // TODO:
        // 1. Eliminate the clone via `.to_vec()` but then it's hard to deserialize
        // 2. We need to switch AWAY from Borsh to something that can be easily digested
        //    by EVM and other chains
        let (payload_without_accounts, solana_accounts) =
            borsh::from_slice::<(Vec<u8>, Vec<SolanaAccountRepr>)>(data).unwrap();
        Self::new_with_cow(Cow::Owned(payload_without_accounts), solana_accounts)
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
}

/// Representation of a Solana account in a way that can be easily serialized
/// for Payload consumption.
///
/// This is the expected data type that will be used to represent Solana
/// accounts in the serilaized payload format.
///
/// Utility methods are provided to encode and decode the representation.
#[derive(Debug, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct SolanaAccountRepr(pub AccountMeta);

impl BorshSerialize for SolanaAccountRepr {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(&self.0.pubkey.to_bytes())?;
        writer.write_all(&[self.0.is_signer as u8 | (self.0.is_writable as u8) << 1])?;
        Ok(())
    }
}

impl BorshDeserialize for SolanaAccountRepr {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut key = [0u8; 32];
        reader.read_exact(&mut key)?;

        let mut flags = [0u8; 1];
        reader.read_exact(&mut flags)?;

        let is_signer = flags[0] & 1 == 1;
        let is_writable = flags[0] >> 1 == 1;

        Ok(SolanaAccountRepr(AccountMeta {
            pubkey: Pubkey::from(key),
            is_signer,
            is_writable,
        }))
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
        SolanaAccountRepr(AccountMeta {
            pubkey: *account.key,
            is_signer: account.is_signer,
            is_writable: account.is_writable,
        })
    }
}

impl<'a> From<&'a AccountMeta> for SolanaAccountRepr {
    fn from(value: &'a AccountMeta) -> Self {
        SolanaAccountRepr(value.clone())
    }
}
impl From<AccountMeta> for SolanaAccountRepr {
    fn from(value: AccountMeta) -> Self {
        SolanaAccountRepr(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solana_account_repr_round_trip() {
        let key = solana_program::pubkey::Pubkey::new_unique();
        let mut lamports = 100;
        let account = solana_program::account_info::AccountInfo::new(
            &key,
            true,
            false,
            &mut lamports,
            &mut [],
            &key,
            false,
            0,
        );
        let repr = SolanaAccountRepr::from(&account);
        let repr_encoded = borsh::to_vec(&repr).unwrap();
        let repr2 = borsh::from_slice(&repr_encoded).unwrap();
        assert_eq!(repr, repr2);
    }
    #[test]
    fn account_serialization() {
        let accounts = &[
            SolanaAccountRepr::from(AccountMeta::new_readonly(
                solana_program::pubkey::Pubkey::new_unique(),
                true,
            )),
            SolanaAccountRepr::from(AccountMeta::new_readonly(
                solana_program::pubkey::Pubkey::new_unique(),
                false,
            )),
            SolanaAccountRepr::from(AccountMeta::new(
                solana_program::pubkey::Pubkey::new_unique(),
                false,
            )),
            SolanaAccountRepr::from(AccountMeta::new(
                solana_program::pubkey::Pubkey::new_unique(),
                true,
            )),
        ];

        let encoded = borsh::to_vec::<Vec<SolanaAccountRepr>>(&accounts.to_vec()).unwrap();
        let decoded = borsh::from_slice::<Vec<SolanaAccountRepr>>(&encoded).unwrap();

        assert_eq!(accounts, decoded.as_slice());
    }

    #[test]
    fn solana_payload_round_trip_using_account_repr() {
        let key = solana_program::pubkey::Pubkey::new_unique();
        let accounts = &[
            AccountMeta::new_readonly(key, true),
            AccountMeta::new_readonly(key, false),
            AccountMeta::new(key, false),
            AccountMeta::new(key, true),
        ];
        let payload_without_accounts = vec![1, 2, 3];
        let repr = DataPayload::new(payload_without_accounts.as_slice(), accounts);
        let repr_encoded = repr.encode();
        let repr2 = DataPayload::decode(&repr_encoded);
        assert_eq!(repr, repr2);
        assert_eq!(repr2.account_meta(), accounts);
    }

    #[test]
    fn axelar_payload_round_trip() {
        let key = solana_program::pubkey::Pubkey::new_unique();
        let mut lamports = 100;
        let account = solana_program::account_info::AccountInfo::new(
            &key,
            true,
            false,
            &mut lamports,
            &mut [],
            &key,
            false,
            0,
        );
        let payload_without_accounts = vec![1, 2, 3];
        let accounts = &[account];
        let payload = DataPayload::new(payload_without_accounts.as_slice(), accounts);
        let axelar_payload_encoded = payload.encode();
        let axelar_payload_decoded = DataPayload::decode(&axelar_payload_encoded);

        assert_eq!(payload, axelar_payload_decoded);
        assert_eq!(payload.hash(), axelar_payload_decoded.hash());
    }

    #[test]
    fn account_info_conversions() {
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
            assert_eq!(repr.0.is_signer, *is_singer, "Signer flag is gone!");
            assert_eq!(repr.0.is_writable, *is_writer, "Writable flag is gone!");
            assert_eq!(repr.0.pubkey, key, "Pubkey does not match!");
            let encoded = borsh::to_vec(&repr).unwrap();
            let decoded = borsh::from_slice::<SolanaAccountRepr>(&encoded).unwrap();

            assert_eq!(repr, decoded, "Round trip failed!");
        }
    }
}
