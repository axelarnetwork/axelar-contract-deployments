mod abi_encoding;
mod borsh_encoding;

use core::mem::size_of;
use std::borrow::Cow;

use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use crate::{AxelarMessagePayload, PayloadError, SolanaAccountRepr};
impl<'payload> AxelarMessagePayload<'payload> {
    /// Encode the payload
    ///
    /// # Errors
    /// - if any of the encoding schemes fail
    pub fn encode(&self) -> Result<Vec<u8>, PayloadError> {
        match self.encoding_scheme {
            EncodingScheme::Borsh => self.encode_borsh(),
            EncodingScheme::AbiEncoding => self.encode_abi_encoding(),
        }
    }

    /// Decode the payload from byte slice
    ///
    /// # Errors
    /// - if the encoding scheme is not valid
    pub fn decode(data: &'payload [u8]) -> Result<Self, PayloadError> {
        let (encoding_scheme, data) = data
            .split_first()
            .ok_or(PayloadError::InvalidEncodingScheme)?;

        let encoding_scheme =
            EncodingScheme::from_u8(*encoding_scheme).ok_or(PayloadError::InvalidEncodingScheme)?;
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

    fn encoding_scheme_prefixed_array(&self) -> Result<Vec<u8>, PayloadError> {
        let mut writer_vec =
            Vec::<u8>::with_capacity(
                // This might not be the exact size, but it's a good approximation
                // Ideally we calculate the size of data before writing it.
                // Could be achieved with a build.rs script that generates the size of the data for
                // each encoding type.
                size_of::<u8>() // encoding scheme
                    .saturating_add(
                        size_of::<u8>().saturating_mul(self.payload_without_accounts.len()),
                    )
                    .saturating_add(
                        size_of::<SolanaAccountRepr>().saturating_mul(self.solana_accounts.len()),
                    ),
            );

        writer_vec.push(
            self.encoding_scheme
                .to_u8()
                .ok_or(PayloadError::InvalidEncodingScheme)?,
        );
        Ok(writer_vec)
    }
}

/// List of encoding schemes that can be used to encode the payload.
///
/// It is expected that this is the first byte of the payload
#[repr(u8)]
#[derive(PartialEq, Debug, Eq, Clone, Copy, FromPrimitive, ToPrimitive)]
#[non_exhaustive]
pub enum EncodingScheme {
    /// Encoding of the payload using Borsh
    Borsh = 0,
    /// Encoding of the payload using ABI (EVM) encoding
    AbiEncoding = 1,
}

#[cfg(test)]
mod tests {
    use solana_program::instruction::AccountMeta;

    use super::*;

    pub(crate) fn account_fixture() -> [SolanaAccountRepr; 4] {
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

    pub(crate) fn account_fixture_2() -> SolanaAccountRepr {
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
        for scheme in &[EncodingScheme::Borsh, EncodingScheme::AbiEncoding] {
            // Setup
            let scheme_as_u8 = scheme.to_u8().unwrap().to_be();

            // Action
            let scheme_from_u8 = EncodingScheme::from_u8(scheme_as_u8).unwrap();

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
            let repr =
                AxelarMessagePayload::new(payload_without_accounts.as_slice(), &accounts, encoding);

            // Action
            let repr_encoded = repr.encode().unwrap();
            let repr2 = AxelarMessagePayload::decode(&repr_encoded).unwrap();

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
            let payload =
                AxelarMessagePayload::new(payload_without_accounts.as_slice(), accounts, encoding);

            // Action
            let axelar_payload_encoded = payload.encode().unwrap();
            let axelar_payload_decoded =
                AxelarMessagePayload::decode(&axelar_payload_encoded).unwrap();

            // Assert
            assert_eq!(payload, axelar_payload_decoded);
            assert_eq!(
                payload.hash().unwrap(),
                axelar_payload_decoded.hash().unwrap()
            );
        }
    }
}
