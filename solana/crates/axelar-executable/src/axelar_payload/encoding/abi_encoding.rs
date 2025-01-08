use alloy_sol_types::{
    abi::{
        token::{DynSeqToken, PackedSeqToken, WordToken},
        Decoder,
    },
    sol, SolValue,
};

use crate::{AxelarMessagePayload, PayloadError, SolanaAccountRepr};

sol! {
    #[repr(C)]
    struct SolanaGatewayPayload {
        bytes execute_payload;
        SolanaAccountRepr[] accounts;
    }
}

impl<'payload> AxelarMessagePayload<'payload> {
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
        let mut writer_vec = self.encoding_scheme_prefixed_array()?;
        let gateway_payload = SolanaGatewayPayload {
            execute_payload: self.payload_without_accounts.to_vec().into(),
            accounts: self.solana_accounts.clone(),
        };

        let res = gateway_payload.abi_encode_params();

        // This is unoptimal because we allocate 2 vectors and then move the data from
        // one to the other.
        writer_vec.extend(&res);

        Ok(writer_vec)
    }

    /// Decodes ABI-encoded data with zero-copy payload handling.
    ///
    /// # Implementation Note
    /// Originally used full alloy decoding, but refactored for zero-copy of large payloads
    /// while maintaining copies only for the small `SolanaAccountRepr` structs.
    ///
    /// # Debug Verification
    /// Debug builds verify our manual decoding against alloy's full (but allocating) decode.
    pub(super) fn decode_abi_encoding(
        data: &'payload [u8],
    ) -> Result<(&'payload [u8], Vec<SolanaAccountRepr>), PayloadError> {
        let (payload, accounts) = extract_payload_silce_and_solana_accounts(data)?;

        // Verify our implementation matches alloy's copying/owned decode
        #[cfg(debug_assertions)]
        {
            let SolanaGatewayPayload {
                execute_payload: allocated_payload,
                accounts: allocated_accounts,
            } = SolanaGatewayPayload::abi_decode_params(data, true)?;

            debug_assert_eq!(payload, allocated_payload.to_vec(), "bad payload");
            debug_assert_eq!(accounts, allocated_accounts, "bad accounts");
        }

        Ok((payload, accounts))
    }
}

/// Performs manual decoding of an ABI-encoded `SolanaGatewayPayload` into its constituent parts.
///
/// The main motivation for this function is to avoid heap allocation of the payload bytes.
/// It achieves this by returning a slice into the original `data` buffer for the payload
/// field, while only allocating memory for the smaller account metadata structures.
///
/// # ABI Structure
/// Decodes `data` conforming to `SolanaGatewayPayload`, which consists of a bytes field
/// followed by an array of account metadata records.
///
/// # Arguments
/// * `data` - ABI-encoded payload bytes, excluding the `EncodingScheme` byte
///
/// # Returns
/// * A tuple containing:
///   - A slice of the original payload bytes (zero-copy)
///   - A vector of decoded Solana account metadata (heap-allocated)
#[inline]
fn extract_payload_silce_and_solana_accounts(
    data: &[u8],
) -> Result<(&[u8], Vec<SolanaAccountRepr>), PayloadError> {
    let mut decoder = Decoder::new(data, true);
    let decoded_sequence = decoder
        .decode_sequence::<<SolanaGatewayPayload as alloy_sol_types::SolType>::Token<'_>>()?;

    // The payload bytes are packed inside the first token, and we can use it directly.
    // Account info is listed inside the second token.
    let (PackedSeqToken(payload_slice), DynSeqToken(account_words)) = decoded_sequence;

    // Process each account's data, which has three components: pubkey, signer and write status
    // They are all represented by a single word (u256, which has 32 bytes).
    let mut accounts = Vec::with_capacity(account_words.len());
    for (WordToken(pubkey_token), WordToken(signer), WordToken(writable)) in account_words {
        accounts.push(SolanaAccountRepr {
            pubkey: pubkey_token,
            is_signer: signer.last() == Some(&1),
            is_writable: writable.last() == Some(&1),
        });
    }

    Ok((payload_slice, accounts))
}

#[cfg(test)]
mod tests {
    use evm_contracts_rs::contracts::example_encoder::ExampleEncoder;
    use evm_contracts_rs::ethers;
    use evm_contracts_test_suite::chain::TestBlockchain;
    use evm_contracts_test_suite::ContractMiddleware;
    use solana_program::instruction::AccountMeta;

    use super::*;
    use crate::axelar_payload::encoding::tests::{account_fixture, account_fixture_2};

    #[test]
    fn solana_account_repr_round_trip_abi() {
        let repr = account_fixture_2();
        let repr_encoded = repr.abi_encode();
        let repr2 = SolanaAccountRepr::abi_decode(&repr_encoded, true).unwrap();
        assert_eq!(repr, repr2);
    }

    #[test]
    fn account_serialization_abi() {
        let accounts = account_fixture().to_vec();
        let encoded = accounts.abi_encode();
        let decoded = Vec::<SolanaAccountRepr>::abi_decode(&encoded, true).unwrap();

        assert_eq!(accounts, decoded);
    }

    #[rstest::rstest]
    #[timeout(core::time::Duration::from_secs(5))]
    #[test_log::test(tokio::test)]
    async fn abi_encode() {
        // Setup
        let (accounts, evm_account_repr) = utils::evm_accounts_fixture();
        let payload_without_accounts = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let canonical_payload = AxelarMessagePayload::new(
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

        let payload_redecoded = AxelarMessagePayload::decode(evm_encoded_payload.as_ref()).unwrap();

        // Assert
        assert_eq!(evm_encoded_payload.to_vec(), canonical_payload_encoded);
        assert_eq!(payload_redecoded, canonical_payload);
    }

    #[rstest::rstest]
    #[timeout(core::time::Duration::from_secs(5))]
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
    #[timeout(core::time::Duration::from_secs(5))]
    #[test_log::test(tokio::test)]
    async fn abi_decode() {
        // Setup
        let (accounts, evm_account_repr) = utils::evm_accounts_fixture();
        let payload_without_accounts = vec![1, 2, 3];
        let canonical_payload = AxelarMessagePayload::new(
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

        pub(crate) async fn chain_setup() -> (ExampleEncoder<ContractMiddleware>, TestBlockchain) {
            let evm_chain = TestBlockchain::new();
            let alice = evm_chain.construct_provider_with_signer(0);
            let contract: ExampleEncoder<ContractMiddleware> =
                alice.deploy_example_encoder().await.unwrap();
            (contract, evm_chain)
        }

        pub(crate) fn evm_accounts_fixture() -> (
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
