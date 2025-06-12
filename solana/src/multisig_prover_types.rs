//! Types from multisig_prover::msg module that we need in our codebase
//! This is a simplified version that only contains what we need to compile

use axelar_solana_encoding::types::pubkey::PublicKey;
use axelar_wasm_std::nonempty::Uint128;
use cosmwasm_std::HexBinary;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

// Extension trait to add u128() method to Uint128
pub(crate) trait Uint128Extensions {
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
pub(crate) enum ProofStatus {
    Pending,
    Completed { execute_data: Vec<u8> },
}

/// The response from querying a proof
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) struct ProofResponse {
    pub(crate) status: ProofStatus,
}

/// The public key of a signer
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) struct Signer {
    pub(crate) pub_key: Pubkey,
    pub(crate) weight: Uint128,
}

/// The public key of a signer
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) enum Pubkey {
    Secp256k1(HexBinary),
    Ed25519(HexBinary),
    #[serde(rename = "ecdsa")]
    Ecdsa(HexBinary),
}

impl TryFrom<Pubkey> for PublicKey {
    type Error = eyre::Report;

    fn try_from(pk: Pubkey) -> Result<Self, Self::Error> {
        match pk {
            Pubkey::Secp256k1(bytes) => {
                let arr: [u8; 33] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| eyre::eyre!("Invalid Secp256k1 key length"))?;
                Ok(PublicKey::Secp256k1(arr))
            }
            Pubkey::Ed25519(bytes) => {
                let arr: [u8; 32] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| eyre::eyre!("Invalid Ed25519 key length"))?;
                Ok(PublicKey::Ed25519(arr))
            }
            Pubkey::Ecdsa(bytes) => {
                let arr: [u8; 33] = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| eyre::eyre!("Invalid Ecdsa key length"))?;
                Ok(PublicKey::Secp256k1(arr)) // Ecdsa is treated as Secp256k1 in axelar_solana_encoding
            }
        }
    }
}

/// A set of verifiers
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) struct VerifierSet {
    pub(crate) signers: std::collections::BTreeMap<String, Signer>,
    pub(crate) threshold: Uint128,
    pub(crate) created_at: u64,
}

/// The response from querying a verifier set
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub(crate) struct VerifierSetResponse {
    pub(crate) verifier_set: VerifierSet,
}

/// The query message for the multisig prover
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QueryMsg {
    CurrentVerifierSet,
    Proof { multisig_session_id: u64 },
}

/// A module to replicate the structure of multisig_prover::msg
pub(crate) mod msg {
    pub(crate) use super::*;
}
