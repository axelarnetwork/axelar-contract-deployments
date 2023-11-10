//! Code copied from the multisig contract, trimmed down to just the used parts.
//!
//! This module only exists because the mock prover doesn't send a submessage to the multicall
//! contract.
//!
//! For some reason, importing the axelar-amplifier `multicall` crate caused wasm compilation errors
//! due to duplicate symbols.

use crate::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, HexBinary, Uint64};
use enum_display_derive::Display;
use serde::{de::Error, Deserialize, Deserializer};
use serde_json::to_string;
use std::collections::HashMap;
use std::fmt::Display;

const ED25519_PUBKEY_LEN: usize = 32;

#[cw_serde]
#[derive(Copy, Display)]
pub enum KeyType {
    // Ecdsa,
    Ed25519,
}
#[cw_serde]
pub struct MsgToSign(HexBinary);
impl MsgToSign {
    pub fn unchecked(hex: HexBinary) -> Self {
        Self(hex)
    }
}

impl From<MsgToSign> for HexBinary {
    fn from(original: MsgToSign) -> Self {
        original.0
    }
}

pub enum MultiSigEvent {
    // Emitted when a new signing session is open
    SigningStarted {
        session_id: Uint64,
        key_id: KeyID,
        pub_keys: HashMap<String, PublicKey>,
        msg: MsgToSign,
    },

    // Emitted when a signing session was completed
    SigningCompleted {
        session_id: Uint64,
        completed_at: u64,
    },
}

impl From<MultiSigEvent> for cosmwasm_std::Event {
    fn from(other: MultiSigEvent) -> Self {
        match other {
            MultiSigEvent::SigningStarted {
                session_id,
                key_id,
                pub_keys,
                msg,
            } => cosmwasm_std::Event::new("signing_started")
                .add_attribute("session_id", session_id)
                .add_attribute(
                    "key_id",
                    to_string(&key_id).expect("violated invariant: key id is not serializable"),
                )
                .add_attribute(
                    "pub_keys",
                    to_string(&pub_keys)
                        .expect("violated invariant: pub_keys are not serializable"),
                )
                .add_attribute("msg", HexBinary::from(msg).to_hex()),

            MultiSigEvent::SigningCompleted {
                session_id,
                completed_at,
            } => cosmwasm_std::Event::new("signing_completed")
                .add_attribute("session_id", session_id)
                .add_attribute("completed_at", completed_at.to_string()),
        }
    }
}

#[cw_serde]
pub struct KeyID {
    pub owner: Addr,
    pub subkey: String,
}

#[cw_serde]
#[derive(Ord, PartialOrd, Eq)]
pub enum PublicKey {
    #[serde(deserialize_with = "deserialize_ed25519_key")]
    Ed25519(HexBinary),
}

fn deserialize_ed25519_key<'de, D>(deserializer: D) -> Result<HexBinary, D::Error>
where
    D: Deserializer<'de>,
{
    let pk: HexBinary = Deserialize::deserialize(deserializer)?;
    PublicKey::try_from((KeyType::Ed25519, pk.clone()))
        .map_err(|e| D::Error::custom(format!("failed to deserialize public key: {}", e)))?;
    Ok(pk)
}

impl TryFrom<(KeyType, HexBinary)> for PublicKey {
    type Error = ContractError;

    fn try_from((key_type, pub_key): (KeyType, HexBinary)) -> Result<Self, Self::Error> {
        match key_type {
            KeyType::Ed25519 => {
                if pub_key.len() != ED25519_PUBKEY_LEN {
                    return Err(ContractError::InvalidPublicKeyFormat {
                        reason: "Invalid input length".into(),
                    });
                }
                Ok(PublicKey::Ed25519(pub_key))
            }
        }
    }
}
