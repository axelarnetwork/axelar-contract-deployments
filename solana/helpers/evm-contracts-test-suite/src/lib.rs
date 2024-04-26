//! # EVM Contracts test suite
//! This crate contains utilities for spinning up a local EVM testnet and
//! deploying contracts on it.

#![warn(missing_docs, unreachable_pub, unused_crate_dependencies)]
#![deny(unused_must_use, rust_2018_idioms)]
#![doc(test(
    no_crate_inject,
    attr(deny(warnings, rust_2018_idioms), allow(dead_code, unused_variables))
))]

use std::sync::Arc;

use ethers::core::k256::ecdsa::SigningKey;
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Provider};
use ethers::signers::{LocalWallet, Wallet};
pub use {ethers, evm_contracts_rs};
pub mod chain;
mod deployments;

/// A wrapper around the `SignerMiddleware` that provides some extra helpers
pub struct EvmSigner {
    /// The signer middleware
    pub signer: Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
    /// The local wallet
    /// Sometimes can come in handy because the signer middleware does not
    /// expose the wallet.
    pub walelt: LocalWallet,
}

/// Utility type for the contract middleware.
/// This type is used for when we instantiate new contract instances
pub type ContractMiddleware = SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>;

/// Utilities for interacting with the Axelar EVM contracts
pub mod evm_operators {
    use std::ops::Range;

    use ethers::abi::{encode_packed, Tokenizable};
    use ethers::signers::Signer;

    use crate::chain::TestBlockchain;

    /// Represents an operator in the `AxelarAuthWeighted` contract.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Operator {
        /// The wallet of the operator
        pub wallet: ethers::signers::LocalWallet,
        /// The weight of the operator
        pub weight: ethers::types::U256,
    }

    /// Represents an operator set in the `AxelarAuthWeighted` contract.
    #[derive(Debug, Clone, PartialEq)]
    pub struct OperatorSet {
        /// The threshold for the operator set
        pub threshold: ethers::types::U256,
        /// The operators in the set
        pub operators: Vec<Operator>,
    }

    /// Create a new operator set using the given range of indices which map to
    /// the wallets available in the test-blockchain node.
    pub fn create_operator_set(
        chain: &TestBlockchain,
        range: Range<usize>,
    ) -> crate::evm_operators::OperatorSet {
        let mut operators = range
            .map(|x| chain.construct_provider_with_signer(x))
            .map(|x| crate::evm_operators::Operator {
                wallet: x.walelt,
                weight: 2.into(),
            })
            .collect::<Vec<_>>();

        sort_by_address(&mut operators);

        crate::evm_operators::OperatorSet {
            threshold: (operators.len() * 2).into(),
            operators,
        }
    }

    /// Builds a command batch for the `AxelarAuthWeighted` contract.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const buildCommandBatch = (chainId, commandIDs, commandNames, commands) =>
    ///     arrayify(defaultAbiCoder.encode(['uint256', 'bytes32[]', 'string[]', 'bytes[]'], [chainId, commandIDs, commandNames, commands]));
    /// ```
    pub fn build_command_batch(
        chain_id: u64,
        command_ids: &[[u8; 32]],
        command_names: Vec<String>,
        commands: Vec<Vec<u8>>,
    ) -> Vec<u8> {
        let command_ids = command_ids
            .iter()
            .map(|x| x.to_vec())
            .map(ethers::abi::Token::FixedBytes)
            .collect::<Vec<_>>();
        let command_names = command_names
            .into_iter()
            .map(ethers::abi::Token::String)
            .collect::<Vec<_>>();
        let commands = commands
            .into_iter()
            .map(ethers::abi::Token::Bytes)
            .collect::<Vec<_>>();

        ethers::abi::encode(&[
            ethers::abi::Token::Uint(ethers::types::U256::from(chain_id)),
            ethers::abi::Token::Array(command_ids),
            ethers::abi::Token::Array(command_names),
            ethers::abi::Token::Array(commands),
        ])
    }

    /// Builds a weighted auth deploy param for the `AxelarAuthWeighted`
    /// contract.
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getWeightedAuthDeployParam = (operatorSets, weights, thresholds) => {
    ///     return operatorSets.map((operators, i) =>
    ///         defaultAbiCoder.encode(['address[]', 'uint256[]', 'uint256'], [getAddresses(operators), weights[i], thresholds[i]]),
    ///     );
    /// };
    /// ```
    pub fn get_weighted_auth_deploy_param(operator_sets: &[OperatorSet]) -> ethers::abi::Token {
        let sets = operator_sets
            .iter()
            .map(|operator_set| {
                let operators = operator_set
                    .operators
                    .iter()
                    .map(|x| x.wallet.address())
                    .collect::<Vec<_>>();
                let weights = operator_set
                    .operators
                    .iter()
                    .map(|x| x.weight)
                    .collect::<Vec<_>>();

                let tokens = &[
                    operators.into_token(),
                    weights.into_token(),
                    operator_set.threshold.into_token(),
                ];
                let res = ethers::abi::encode(tokens);
                ethers::abi::Token::Bytes(res)
            })
            .collect::<Vec<_>>();
        ethers::abi::Token::Array(sets)
    }

    /// Builds a transfer weighted operatorship command for the
    /// `Gateway` contract.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getTransferWeightedOperatorshipCommand = (newOperators, newWeights, threshold) =>
    ///      defaultAbiCoder.encode(
    ///          ['address[]', 'uint256[]', 'uint256'],
    ///          [sortBy(newOperators, (address) => address.toLowerCase()), newWeights,  threshold],
    /// );
    /// ```
    pub fn get_transfer_weighted_operatorship_command(
        new_operators: &mut OperatorSet,
        threshold: ethers::types::U256,
    ) -> Vec<u8> {
        sort_by_address(&mut new_operators.operators);
        let operators = new_operators
            .operators
            .iter()
            .map(|x| x.wallet.address())
            .map(ethers::abi::Token::Address)
            .collect::<Vec<_>>();
        let weights = new_operators
            .operators
            .iter()
            .map(|x| x.weight)
            .map(ethers::abi::Token::Uint)
            .collect::<Vec<_>>();
        ethers::abi::encode(&[
            ethers::abi::Token::Array(operators),
            ethers::abi::Token::Array(weights),
            ethers::abi::Token::Uint(threshold),
        ])
    }

    /// Encodes an approve contract call for the `Gateway` contract.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getApproveContractCall = (sourceChain, source, destination, payloadHash, sourceTxHash, sourceEventIndex) =>
    ///     defaultAbiCoder.encode(
    ///         ['string', 'string', 'address', 'bytes32', 'bytes32', 'uint256'],
    ///         [sourceChain, source, destination, payloadHash, sourceTxHash, sourceEventIndex],
    ///     );
    /// ```
    pub fn get_approve_contract_call(
        source_chain: String,
        source: String,
        destination: ethers::types::Address,
        payload_hash: [u8; 32],
        source_tx_hash: [u8; 32],
        source_event_index: ethers::types::U256,
    ) -> Vec<u8> {
        ethers::abi::encode(&[
            ethers::abi::Token::String(source_chain),
            ethers::abi::Token::String(source),
            ethers::abi::Token::Address(destination),
            ethers::abi::Token::FixedBytes(payload_hash.to_vec()),
            ethers::abi::Token::FixedBytes(source_tx_hash.to_vec()),
            ethers::abi::Token::Uint(source_event_index),
        ])
    }

    /// Signs a payload with the given operator set.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getWeightedSignaturesProof = async (data, operators, weights, threshold, signers) => {
    ///     const hash = arrayify(keccak256(data));
    ///     const signatures = await Promise.all(
    ///         sortBy(signers, (wallet) => wallet.address.toLowerCase()).map((wallet) => wallet.signMessage(hash)),
    ///     );
    ///     return defaultAbiCoder.encode(
    ///         ['address[]', 'uint256[]', 'uint256', 'bytes[]'],
    ///         [getAddresses(operators), weights, threshold, signatures],
    ///     );
    /// };
    /// ```
    pub fn get_weighted_signatures_proof(data: &[u8], operator_set: &mut OperatorSet) -> Vec<u8> {
        let hash = ethers::utils::keccak256(data);
        let packed_msg = encode_packed(&[
            ethers::abi::Token::String("\x19Ethereum Signed Message:\n32".to_string()),
            ethers::abi::Token::FixedBytes(hash.to_vec()),
        ])
        .unwrap();
        let hash = ethers::utils::keccak256(packed_msg);
        sort_by_address(&mut operator_set.operators);
        let signatures = operator_set
            .operators
            .iter()
            .map(|operator| {
                let signature = operator.wallet.sign_hash(hash.into()).unwrap();
                signature.to_vec()
            })
            .map(ethers::abi::Token::Bytes)
            .collect::<Vec<_>>();
        let weights = operator_set
            .operators
            .iter()
            .map(|operator| operator.weight)
            .map(ethers::abi::Token::Uint)
            .collect::<Vec<_>>();
        let operators = operator_set
            .operators
            .iter()
            .map(|operator| operator.wallet.address())
            .map(ethers::abi::Token::Address)
            .collect::<Vec<_>>();
        ethers::abi::encode(&[
            ethers::abi::Token::Array(operators),
            ethers::abi::Token::Array(weights),
            ethers::abi::Token::Uint(operator_set.threshold),
            ethers::abi::Token::Array(signatures),
        ])
    }

    /// Signs a payload with the given operator set, concatenates the signature
    /// with the payload.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getSignedWeightedExecuteInput = async (data, operators, weights, threshold, signers) =>
    /// defaultAbiCoder.encode(['bytes', 'bytes'], [data, await getWeightedSignaturesProof(data, operators, weights, threshold, signers)]);
    /// ```
    pub fn get_signed_weighted_execute_input(
        data: Vec<u8>,
        operators: &mut OperatorSet,
    ) -> Vec<u8> {
        let proof = crate::evm_operators::get_weighted_signatures_proof(&data, operators);
        ethers::abi::encode(&[
            ethers::abi::Token::Bytes(data),
            ethers::abi::Token::Bytes(proof),
        ])
    }

    pub(crate) fn sort_by_address(operators: &mut [Operator]) {
        operators.sort_by(|a, b| a.wallet.address().cmp(&b.wallet.address()));
    }
}
