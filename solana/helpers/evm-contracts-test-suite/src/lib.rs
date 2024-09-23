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
use ethers::signers::{LocalWallet, Signer, Wallet};
pub use {ethers, evm_contracts_rs};
pub mod chain;
mod deployments;
pub use deployments::{await_receipt, get_domain_separator};

/// A wrapper around the `SignerMiddleware` that provides some extra helpers
#[derive(Clone)]
pub struct EvmSigner {
    /// The signer middleware
    pub signer: Arc<SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>>,
    /// The local wallet
    /// Sometimes can come in handy because the signer middleware does not
    /// expose the wallet.
    pub wallet: LocalWallet,
}

impl std::fmt::Debug for EvmSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EvmSigner")
            .field("address", &self.wallet.address())
            .finish()
    }
}
/// Utility type for the contract middleware.
/// This type is used for when we instantiate new contract instances
pub type ContractMiddleware = SignerMiddleware<Arc<Provider<Http>>, Wallet<SigningKey>>;

/// Utilities for interacting with the Axelar EVM contracts
pub mod evm_weighted_signers {
    use std::ops::Range;

    use ethers::abi::{encode_packed, Tokenizable};
    use ethers::signers::Signer;
    use ethers::types::Address;
    use ethers::utils::keccak256;

    use crate::chain::TestBlockchain;

    /// Represents an operator in the `AxelarAuthWeighted` contract.
    #[derive(Debug, Clone, PartialEq)]
    pub struct WeightedSigner {
        /// The wallet of the operator
        pub wallet: ethers::signers::LocalWallet,
        /// The weight of the operator
        pub weight: ethers::types::U128,
    }

    /// Represents an operator set in the `AxelarAuthWeighted` contract.
    #[derive(Debug, Clone, PartialEq)]
    pub struct WeightedSigners {
        /// The threshold for the operator set
        pub threshold: ethers::types::U128,
        /// Unique nonce represent used to differentiate otherwise unique
        /// operator sets
        pub nonce: [u8; 32],
        /// The operators in the set
        pub signers: Vec<WeightedSigner>,
    }

    /// Create a new operator set using the given range of indices which map to
    /// the wallets available in the test-blockchain node.
    pub fn create_operator_set(
        chain: &TestBlockchain,
        range: Range<usize>,
    ) -> crate::evm_weighted_signers::WeightedSigners {
        let mut operators = range
            .map(|x| chain.construct_provider_with_signer(x))
            .map(|x| crate::evm_weighted_signers::WeightedSigner {
                wallet: x.wallet,
                weight: 2.into(),
            })
            .collect::<Vec<_>>();

        sort_by_address(&mut operators);

        crate::evm_weighted_signers::WeightedSigners {
            threshold: (operators.len() * 2).into(),
            signers: operators,
            nonce: keccak256("123"),
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
    pub fn get_weighted_auth_deploy_param(operator_sets: &[WeightedSigners]) -> ethers::abi::Token {
        let sets = operator_sets
            .iter()
            .map(|operator_set| {
                let operators = operator_set
                    .signers
                    .iter()
                    .map(|x| x.wallet.address())
                    .collect::<Vec<_>>();
                let weights = operator_set
                    .signers
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

    /// Encodes an approve contract call for the `Gateway` contract.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// const getApproveMessageData = (messages) => {
    ///     return defaultAbiCoder.encode(
    ///         [
    ///             'uint8',
    ///             'tuple(string sourceChain, string messageId, string sourceAddress, address contractAddress, bytes32 payloadHash)[] messages',
    ///         ],
    ///         [APPROVE_MESSAGES, messages],
    ///     );
    /// };
    /// ```
    pub fn get_approve_contract_call(
        message: evm_contracts_rs::contracts::axelar_amplifier_gateway::Message,
    ) -> Vec<u8> {
        const APPROVE_MESSAGE: u8 = 0;

        ethers::abi::encode(&[
            ethers::abi::Token::Uint(APPROVE_MESSAGE.into()),
            ethers::abi::Token::Array(
                [ethers::abi::Token::Tuple(
                    [
                        ethers::abi::Token::String(message.source_chain),
                        ethers::abi::Token::String(message.message_id),
                        ethers::abi::Token::String(message.source_address),
                        ethers::abi::Token::Address(message.contract_address),
                        ethers::abi::Token::FixedBytes(message.payload_hash.to_vec()),
                    ]
                    .to_vec(),
                )]
                .to_vec(),
            ),
        ])
    }

    /// Signs a payload with the given operator set.
    ///
    /// ported from the following TypeScript code:
    /// ```typescript
    /// 
    /// const WEIGHTED_SIGNERS_TYPE = 'tuple(tuple(address signer,uint128 weight)[] signers,uint128 threshold,bytes32 nonce)';
    /// const encodeWeightedSigners = (weightedSigners) => {
    ///     return defaultAbiCoder.encode([WEIGHTED_SIGNERS_TYPE], [weightedSigners]);
    /// };
    ///
    /// const encodeWeightedSignersMessage = (data, domainSeparator, weightedSignerHash) => {
    ///     return arrayify(domainSeparator + weightedSignerHash.slice(2) + keccak256(arrayify(data)).slice(2));
    /// };
    ///
    /// const encodeMessageHash = (data, domainSeparator, weightedSignerHash) => {
    ///     return hashMessage(encodeWeightedSignersMessage(data, domainSeparator, weightedSignerHash));
    /// };
    ///
    /// const getWeightedSignersProof = async (data, domainSeparator, weightedSigners, wallets) => {
    ///     const weightedSignerHash = keccak256(encodeWeightedSigners(weightedSigners));
    ///     const message = encodeWeightedSignersMessage(data, domainSeparator, weightedSignerHash);
    ///
    ///     const signatures = await Promise.all(wallets.map((wallet) => wallet.signMessage(message)));
    ///
    ///     return { signers: weightedSigners, signatures };
    /// };  
    /// ```
    pub fn get_weighted_signatures_proof(
        data: &[u8],
        signer_set: &mut WeightedSigners,
        domain_separator: [u8; 32],
    ) -> evm_contracts_rs::contracts::axelar_amplifier_gateway::Proof {
        let weighted_signer_hash =
            keccak256(ethers::abi::encode(&[get_weighted_signers(signer_set)]));
        let packed_msg = encode_packed(&[
            ethers::abi::Token::String("\x19Ethereum Signed Message:\n96".to_string()),
            ethers::abi::Token::FixedBytes(domain_separator.to_vec()),
            ethers::abi::Token::FixedBytes(weighted_signer_hash.to_vec()),
            ethers::abi::Token::FixedBytes(keccak256(data).to_vec()),
        ])
        .unwrap();
        let hash = ethers::utils::keccak256(packed_msg);
        sort_by_address(&mut signer_set.signers);
        let signatures = signer_set
            .signers
            .iter()
            .map(|signer| {
                let signature = signer.wallet.sign_hash(hash.into()).unwrap();
                signature.to_vec()
            })
            .map(|x| x.into())
            .collect::<Vec<_>>();
        evm_contracts_rs::contracts::axelar_amplifier_gateway::Proof {
            signers: evm_contracts_rs::contracts::axelar_amplifier_gateway::WeightedSigners {
                signers: signer_set
                    .signers
                    .iter()
                    .map(|x| {
                        evm_contracts_rs::contracts::axelar_amplifier_gateway::WeightedSigner {
                            signer: x.wallet.address(),
                            weight: x.weight.as_u128(),
                        }
                    })
                    .collect::<Vec<_>>(),
                threshold: signer_set.threshold.as_u128(),
                nonce: signer_set.nonce,
            },
            signatures,
        }
    }

    /// (address operator_, WeightedSigners[] memory signers) =
    /// abi.decode(data, (address, WeightedSigners[]));
    pub fn get_gateway_proxy_setup_signers(
        recent_signer_sets: &[crate::evm_weighted_signers::WeightedSigners],
        operator: Address,
    ) -> Vec<u8> {
        let signer_sets = recent_signer_sets
            .iter()
            .map(get_weighted_signers)
            .collect::<Vec<_>>();
        ethers::abi::encode(&[
            ethers::abi::Token::Address(operator),
            ethers::abi::Token::Array(signer_sets),
        ])
    }

    /// const WEIGHTED_SIGNERS_TYPE = 'tuple(tuple(address signer,uint128
    /// weight)[] signers,uint128 threshold,bytes32 nonce)';
    pub fn get_weighted_signers(
        signer_set: &crate::evm_weighted_signers::WeightedSigners,
    ) -> ethers::abi::Token {
        let signers = signer_set
            .signers
            .iter()
            .map(|x| {
                ethers::abi::Token::Tuple(vec![
                    ethers::abi::Token::Address(x.wallet.address()),
                    ethers::abi::Token::Uint(x.weight.into()),
                ])
            })
            .collect::<Vec<_>>();

        ethers::abi::Token::Tuple(vec![
            ethers::abi::Token::Array(signers),
            ethers::abi::Token::Uint(signer_set.threshold.into()),
            ethers::abi::Token::FixedBytes(signer_set.nonce.into()),
        ])
    }

    pub(crate) fn sort_by_address(operators: &mut [WeightedSigner]) {
        operators.sort_by(|a, b| a.wallet.address().cmp(&b.wallet.address()));
    }
}
