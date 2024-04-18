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
