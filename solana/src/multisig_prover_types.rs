//! Types from multisig_prover::msg module that we need in our codebase
//! This is a simplified version that only contains what we need to compile

use axelar_wasm_std::nonempty::Uint128;
use cosmwasm_std::HexBinary;
use serde::{Deserialize, Serialize};

// Extension trait to add u128() method to Uint128
pub trait Uint128Extensions {
    fn u128(&self) -> u128;
}

// Implement the trait for Uint128
impl Uint128Extensions for Uint128 {
    fn u128(&self) -> u128 {
        self.into_inner().u128()
    }
}

/// The status of a proof
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ProofStatus {
    Pending,
    Completed { execute_data: Vec<u8> },
}

/// The response from querying a proof
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProofResponse {
    pub status: ProofStatus,
}

/// The public key of a signer
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Signer {
    pub pub_key: HexBinary,
    pub weight: Uint128,
}

/// A set of verifiers
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VerifierSet {
    pub signers: std::collections::BTreeMap<String, Signer>,
    pub threshold: Uint128,
    pub created_at: u64,
}

/// The response from querying a verifier set
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct VerifierSetResponse {
    pub verifier_set: VerifierSet,
}

/// The query message for the multisig prover
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CurrentVerifierSet,
    Proof { multisig_session_id: u64 },
}

/// A module to replicate the structure of multisig_prover::msg
pub mod msg {
    pub use super::*;
}
