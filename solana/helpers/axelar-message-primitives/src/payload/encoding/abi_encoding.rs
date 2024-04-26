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
    use evm_contracts_rs::contracts::example_encoder::ExampleEncoder;
    use evm_contracts_rs::ethers;
    use evm_contracts_test_suite::chain::TestBlockchain;
    use evm_contracts_test_suite::ContractMiddleware;

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

    #[rstest::rstest]
    #[timeout(std::time::Duration::from_secs(5))]
    #[test_log::test(tokio::test)]
    async fn abi_encode() {
        // Setup
        let (accounts, evm_account_repr) = utils::evm_accounts_fixture();
        let payload_without_accounts = vec![42, 111];
        let canonical_payload = DataPayload::new(
            payload_without_accounts.as_slice(),
            &accounts,
            crate::EncodingScheme::AbiEncoding,
        );
        let canonical_payload_encoded = canonical_payload.encode().unwrap();
        let (contract, _evm_chain) = utils::chain_setup().await;

        // Action
        let evm_encoded_payload: ethers::types::Bytes = contract
            .encode(
                evm_contracts_rs::contracts::example_encoder::SolanaGatewayPayload {
                    execute_payload: payload_without_accounts.clone().into(),
                    accounts: evm_account_repr,
                },
            )
            .await
            .unwrap();
        let payload_redecoded = DataPayload::decode(evm_encoded_payload.as_ref()).unwrap();

        // Assert
        assert_eq!(evm_encoded_payload.to_vec(), canonical_payload_encoded);
        assert_eq!(payload_redecoded, canonical_payload);
    }

    #[rstest::rstest]
    #[timeout(std::time::Duration::from_secs(5))]
    #[test_log::test(tokio::test)]
    async fn abi_encoding_solidity_roundtrip() {
        // Setup
        let (_accounts, evm_account_repr) = utils::evm_accounts_fixture();
        let payload_without_accounts = vec![42, 111];
        let (contract, _evm_chain) = utils::chain_setup().await;
        let payload = evm_contracts_rs::contracts::example_encoder::SolanaGatewayPayload {
            execute_payload: payload_without_accounts.into(),
            accounts: evm_account_repr,
        };

        // Action
        let evm_encoded_payload: ethers::types::Bytes =
            contract.encode(payload.clone()).await.unwrap();
        let decoded_payload = contract.decode(evm_encoded_payload).await.unwrap();

        // Assert
        assert_eq!(decoded_payload, payload);
    }

    #[rstest::rstest]
    #[timeout(std::time::Duration::from_secs(5))]
    #[test_log::test(tokio::test)]
    async fn abi_decode() {
        // Setup
        let (accounts, evm_account_repr) = utils::evm_accounts_fixture();
        let payload_without_accounts = vec![1, 2, 3];
        let canonical_payload = DataPayload::new(
            payload_without_accounts.as_slice(),
            &accounts,
            crate::EncodingScheme::AbiEncoding,
        );
        let canonical_payload_encoded = canonical_payload.encode().unwrap();
        let (contract, _evm_chain) = utils::chain_setup().await;

        // Action
        let evm_decoded_payload: evm_contracts_rs::contracts::example_encoder::SolanaGatewayPayload = contract
            .decode(canonical_payload_encoded.into())
            .await
            .unwrap();

        // Assert
        assert_eq!(
            evm_decoded_payload.execute_payload.to_vec(),
            canonical_payload.payload_without_accounts.to_vec()
        );
        assert_eq!(evm_decoded_payload.accounts, evm_account_repr);
    }

    mod utils {
        use super::*;

        pub async fn chain_setup() -> (ExampleEncoder<ContractMiddleware>, TestBlockchain) {
            let evm_chain = TestBlockchain::new();
            let alice = evm_chain.construct_provider_with_signer(0);
            let contract: ExampleEncoder<ContractMiddleware> =
                alice.deploy_example_encoder().await.unwrap();
            (contract, evm_chain)
        }

        pub fn evm_accounts_fixture() -> (
            Vec<AccountMeta>,
            Vec<evm_contracts_rs::contracts::example_encoder::SolanaAccountRepr>,
        ) {
            let accounts = account_fixture()
                .into_iter()
                .map(AccountMeta::from)
                .collect::<Vec<_>>();
            let evm_account_repr = accounts
                .clone()
                .into_iter()
                .map(
                    |x| evm_contracts_rs::contracts::example_encoder::SolanaAccountRepr {
                        pubkey: x.pubkey.to_bytes(),
                        is_signer: x.is_signer,
                        is_writable: x.is_writable,
                    },
                )
                .collect::<Vec<_>>();
            (accounts, evm_account_repr)
        }
    }
}
