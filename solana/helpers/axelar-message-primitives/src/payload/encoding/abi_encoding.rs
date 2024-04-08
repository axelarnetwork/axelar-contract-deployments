use borsh::BorshDeserialize;
use ethers_core::abi::{ParamType, Token, Tokenizable};
use solana_program::instruction::AccountMeta;
use solana_program::pubkey::Pubkey;

use crate::{DataPayload, PayloadError, SolanaAccountRepr};

impl<'payload> DataPayload<'payload> {
    /// Encodes the payload using the ABI encoding scheme.
    ///
    /// The payload is encoded the following way:
    /// - single byte indicating the encoding scheme.
    /// - encoded: The first element is the payload without the accounts.
    /// - encoded: The second element is the list of Solana accounts.
    ///
    /// FIXME: this function is very inefficient because it allocates up to 5
    /// vectors.
    pub(super) fn encode_abi_encoding(&self) -> Result<Vec<u8>, PayloadError> {
        let mut writer_vec = self.encoding_scheme_prefixed_array();
        let payload_bytes = Token::Bytes(self.payload_without_accounts.as_ref().to_vec());
        let solana_accounts = VecSolanaAccountRepr(self.solana_accounts.to_vec()).into_token();
        let res = ethers_core::abi::encode(&[payload_bytes, solana_accounts]);

        // This is unoptimal because we allocate 2 vectors and then move the data from
        // one to the other.
        writer_vec.extend(res);

        Ok(writer_vec)
    }

    pub(super) fn decode_abi_encoding(
        data: &'payload [u8],
    ) -> Result<(Vec<u8>, Vec<SolanaAccountRepr>), PayloadError> {
        let mut tokens = ethers_core::abi::decode(
            &[
                ParamType::Bytes,
                ParamType::Array(Box::new(ParamType::Tuple(vec![
                    ParamType::FixedBytes(32),
                    ParamType::Bool,
                    ParamType::Bool,
                ]))),
            ],
            data,
        )?
        .into_iter();

        let payload_bytes = tokens
            .next()
            .ok_or(PayloadError::AbiTokenNotPresent)?
            .into_bytes()
            .ok_or(PayloadError::AbiTokenNotPresent)?;
        let solana_accounts = VecSolanaAccountRepr::from_token(
            tokens.next().ok_or(PayloadError::AbiTokenNotPresent)?,
        )?
        .0;

        Ok((payload_bytes, solana_accounts))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[repr(transparent)]
struct VecSolanaAccountRepr(Vec<SolanaAccountRepr>);

impl Tokenizable for VecSolanaAccountRepr {
    fn from_token(
        token: ethers_core::abi::Token,
    ) -> Result<Self, ethers_core::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        let tokens = token
            .into_array()
            .ok_or_else(|| create_invalid_output_type("token array not found"))?;
        let mut accounts = Vec::with_capacity(tokens.len());
        for token in tokens {
            accounts.push(SolanaAccountRepr::from_token(token)?);
        }
        Ok(Self(accounts))
    }

    fn into_token(self) -> ethers_core::abi::Token {
        let tokens = self
            .0
            .into_iter()
            .map(|account| account.into_token())
            .collect();
        ethers_core::abi::Token::Array(tokens)
    }
}

impl Tokenizable for SolanaAccountRepr {
    fn from_token(
        token: ethers_core::abi::Token,
    ) -> Result<Self, ethers_core::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        let mut tokens = token
            .into_tuple()
            .ok_or_else(|| create_invalid_output_type("token tuple not found"))?
            .into_iter();
        let pubkey = tokens
            .next()
            .and_then(|x| x.into_fixed_bytes())
            .map(|x| Pubkey::try_from_slice(&x))
            .ok_or_else(|| create_invalid_output_type("Param not fixed bytes"))?
            .map_err(|_| create_invalid_output_type("Invalid pubkey"))?;
        let is_signer = tokens
            .next()
            .and_then(|x| x.into_bool())
            .ok_or_else(|| create_invalid_output_type("Param not bool"))?;
        let is_writable = tokens
            .next()
            .and_then(|x| x.into_bool())
            .ok_or_else(|| create_invalid_output_type("Param not bool"))?;

        Ok(Self(AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        }))
    }

    fn into_token(self) -> ethers_core::abi::Token {
        Token::Tuple(vec![
            Token::FixedBytes(self.0.pubkey.to_bytes().to_vec()),
            Token::Bool(self.0.is_signer),
            Token::Bool(self.0.is_writable),
        ])
    }
}

fn create_invalid_output_type(msg: &str) -> ethers_core::abi::InvalidOutputType {
    ethers_core::abi::InvalidOutputType(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::encoding::tests::{account_fixture, account_fixture_2};

    #[test]
    fn solana_account_repr_round_trip_abi() {
        let repr = account_fixture_2();
        let repr_encoded = repr.clone().into_token();
        let repr2 = SolanaAccountRepr::from_token(repr_encoded).unwrap();
        assert_eq!(repr, repr2);
    }

    #[test]
    fn account_serialization_abi() {
        let accounts = account_fixture().to_vec();
        let encoded = VecSolanaAccountRepr(accounts.clone()).into_token();
        let decoded = VecSolanaAccountRepr::from_token(encoded).unwrap().0;

        assert_eq!(accounts, decoded.as_slice());
    }
}
