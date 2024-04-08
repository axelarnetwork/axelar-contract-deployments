mod abi_encoding;
mod borsh_encoding;

use std::borrow::Cow;
use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};

use crate::{DataPayload, PayloadError, SolanaAccountRepr};
impl<'payload> DataPayload<'payload> {
    /// Encode the payload
    pub fn encode(&self) -> Result<Vec<u8>, PayloadError> {
        match self.encoding_scheme {
            EncodingScheme::Borsh => self.encode_borsh(),
            EncodingScheme::AbiEncoding => self.encode_abi_encoding(),
        }
    }

    /// Decode the payload from byte slice
    pub fn decode(data: &'payload [u8]) -> Result<Self, PayloadError> {
        let (encoding_scheme, data) = data
            .split_first()
            .ok_or(PayloadError::InvalidEncodingScheme)?;

        let encoding_scheme = EncodingScheme::try_from(*encoding_scheme)?;
        let (payload_without_accounts, solana_accounts) = match encoding_scheme {
            EncodingScheme::Borsh => Self::decode_borsh(data)?,
            EncodingScheme::AbiEncoding => Self::decode_abi_encoding(data)?,
        };

        Ok(Self::new_with_cow(
            Cow::Owned(payload_without_accounts),
            solana_accounts,
            encoding_scheme,
        ))
    }

    fn encoding_scheme_prefixed_array(&self) -> Vec<u8> {
        let mut writer_vec = Vec::<u8>::with_capacity(
            // This might not be the exact size, but it's a good approximation
            // Ideally we calcualte the size of data before writing it.
            // Could be achieved with a build.rs script that generates the size of the data for
            // each encoding type.
            size_of::<u8>() // encoding scheme
                    + (size_of::<u8>() * self.payload_without_accounts.len())
                    + (size_of::<SolanaAccountRepr>() * self.solana_accounts.len() ),
        );
        writer_vec.push(self.encoding_scheme.to_u8());
        writer_vec
    }
}

/// List of encoding schemes that can be used to encode the payload.
///
/// It is expected that this is the first byte of the payload
#[repr(u8)]
#[derive(
    PartialEq,
    Debug,
    Eq,
    Clone,
    Copy,
    // We need to derive BorshSerialize and BorshDeserialize so we can use this enum
    // in the AxelarExecutablePayload struct.
    BorshSerialize,
    BorshDeserialize,
)]
#[non_exhaustive]
#[borsh(use_discriminant = true)]
pub enum EncodingScheme {
    Borsh = 0,
    AbiEncoding = 1,
}

impl TryFrom<u8> for EncodingScheme {
    type Error = PayloadError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Borsh),
            1 => Ok(Self::AbiEncoding),
            _ => Err(PayloadError::InvalidEncodingScheme),
        }
    }
}

impl EncodingScheme {
    fn to_u8(self) -> u8 {
        self as u8
    }
}

#[cfg(test)]
mod tests {
    use solana_program::instruction::AccountMeta;

    use super::*;

    pub fn account_fixture() -> [SolanaAccountRepr; 4] {
        [(true, true), (true, false), (false, true), (false, false)].map(
            |(is_signer, is_writer)| {
                let key = solana_program::pubkey::Pubkey::new_unique();
                let mut lamports = 100;
                let account = solana_program::account_info::AccountInfo::new(
                    &key,
                    is_signer,
                    is_writer,
                    &mut lamports,
                    &mut [],
                    &key,
                    false,
                    0,
                );
                SolanaAccountRepr::from(&account)
            },
        )
    }

    pub fn account_fixture_2() -> SolanaAccountRepr {
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
        SolanaAccountRepr::from(&account)
    }

    #[test]
    fn encoding_scheme() {
        for scheme in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding].iter() {
            // Setup
            let scheme_as_u8 = scheme.to_u8().to_be();

            // Action
            let scheme_from_u8 = EncodingScheme::try_from(scheme_as_u8).unwrap();

            // Assert
            assert_eq!(scheme_from_u8, *scheme);
        }
    }

    #[test]
    fn payload_round_trip_many_accounts() {
        for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
            // Setup
            let accounts = account_fixture()
                .into_iter()
                .map(AccountMeta::from)
                .collect::<Vec<_>>();
            let payload_without_accounts = vec![1, 2, 3];
            let repr = DataPayload::new(payload_without_accounts.as_slice(), &accounts, encoding);

            // Action
            let repr_encoded = repr.encode().unwrap();
            let repr2 = DataPayload::decode(&repr_encoded).unwrap();

            // Assert
            assert_eq!(repr, repr2);
            assert_eq!(repr2.account_meta(), accounts);
        }
    }

    #[test]
    fn payload_round_trip_single_account() {
        for encoding in [EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
            // Setup
            let account = account_fixture_2();
            let payload_without_accounts = vec![1, 2, 3];
            let accounts = &[account];
            let payload = DataPayload::new(payload_without_accounts.as_slice(), accounts, encoding);

            // Action
            let axelar_payload_encoded = payload.encode().unwrap();
            dbg!(&axelar_payload_encoded);
            let axelar_payload_decoded = DataPayload::decode(&axelar_payload_encoded).unwrap();

            // Assert
            assert_eq!(payload, axelar_payload_decoded);
            assert_eq!(
                payload.hash().unwrap(),
                axelar_payload_decoded.hash().unwrap()
            );
        }
    }
}
