use std::collections::BTreeMap;
use std::str::FromStr;

use axelar_solana_encoding::types::pubkey::PublicKey;
use axelar_solana_encoding::types::verifier_set::VerifierSet;
use clap::ArgEnum;
use eyre::eyre;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use serde::{Deserialize, Serialize};
use solana_sdk::instruction::Instruction as SolanaInstruction;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::transaction::Transaction as SolanaTransaction;

#[derive(ArgEnum, Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum NetworkType {
    Mainnet,
    Testnet,
    Devnet,
    Localnet,
}

impl FromStr for NetworkType {
    type Err = eyre::Error;

    fn from_str(s: &str) -> eyre::Result<Self> {
        s.contains("mainnet")
            .then_some(NetworkType::Mainnet)
            .or_else(|| s.contains("testnet").then_some(NetworkType::Testnet))
            .or_else(|| s.contains("devnet").then_some(NetworkType::Devnet))
            .or_else(|| s.contains("local").then_some(NetworkType::Localnet))
            .ok_or_else(|| eyre!("Invalid network type: {s}"))
    }
}

pub(crate) struct ChainsInfoFile(pub(crate) String);
impl From<ChainsInfoFile> for String {
    fn from(value: ChainsInfoFile) -> Self {
        value.0
    }
}

impl From<NetworkType> for ChainsInfoFile {
    fn from(value: NetworkType) -> Self {
        match value {
            NetworkType::Mainnet => Self("mainnet.json".to_owned()),
            NetworkType::Testnet => Self("testnet.json".to_owned()),
            NetworkType::Devnet => Self("devnet-amplifier.json".to_owned()),
            NetworkType::Localnet => Self("local.json".to_owned()),
        }
    }
}

pub(crate) struct ChainNameOnAxelar(pub(crate) String);

impl From<NetworkType> for ChainNameOnAxelar {
    fn from(value: NetworkType) -> Self {
        match value {
            NetworkType::Mainnet => Self("solana".to_owned()),
            NetworkType::Testnet => Self("solana-testnet".to_owned()),
            NetworkType::Devnet => Self("solana-devnet".to_owned()),
            NetworkType::Localnet => Self("solana-localnet".to_owned()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SolanaTransactionParams {
    pub(crate) fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) recent_blockhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) nonce_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) nonce_authority: Option<String>,
    pub(crate) blockhash_for_message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SerializableInstruction {
    pub(crate) program_id: String,
    pub(crate) accounts: Vec<SerializableAccountMeta>,
    #[serde(with = "hex::serde")]
    pub(crate) data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SerializableAccountMeta {
    pub(crate) pubkey: String,
    pub(crate) is_signer: bool,
    pub(crate) is_writable: bool,
}

impl TryFrom<&SerializableInstruction> for SolanaInstruction {
    type Error = eyre::Error;

    fn try_from(si: &SerializableInstruction) -> eyre::Result<Self> {
        let program_id = Pubkey::from_str(&si.program_id)?;
        let accounts = si
            .accounts
            .iter()
            .map(|sa| {
                Ok(solana_sdk::instruction::AccountMeta {
                    pubkey: Pubkey::from_str(&sa.pubkey)?,
                    is_signer: sa.is_signer,
                    is_writable: sa.is_writable,
                })
            })
            .collect::<eyre::Result<Vec<_>>>()?;

        Ok(SolanaInstruction {
            program_id,
            accounts,
            data: si.data.clone(),
        })
    }
}

impl From<&SolanaInstruction> for SerializableInstruction {
    fn from(instruction: &SolanaInstruction) -> Self {
        Self {
            program_id: instruction.program_id.to_string(),
            accounts: instruction
                .accounts
                .iter()
                .map(|am| SerializableAccountMeta {
                    pubkey: am.pubkey.to_string(),
                    is_signer: am.is_signer,
                    is_writable: am.is_writable,
                })
                .collect(),
            data: instruction.data.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct UnsignedSolanaTransaction {
    pub(crate) params: SolanaTransactionParams,
    pub(crate) instructions: Vec<SerializableInstruction>,
    pub(crate) signable_message_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct PartialSignature {
    pub(crate) signer_pubkey: String,
    pub(crate) signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct SignedSolanaTransaction {
    pub(crate) unsigned_tx_data: UnsignedSolanaTransaction,
    pub(crate) signatures: Vec<PartialSignature>,
}

/// A wrapper around SolanaTransaction that can be serialized and deserialized
#[derive(Debug, Clone)]
pub(crate) struct SerializableSolanaTransaction {
    pub(crate) transaction: SolanaTransaction,
    pub(crate) params: SolanaTransactionParams,
}

impl SerializableSolanaTransaction {
    pub(crate) fn new(transaction: SolanaTransaction, params: SolanaTransactionParams) -> Self {
        Self {
            transaction,
            params,
        }
    }

    pub(crate) fn to_unsigned(&self) -> UnsignedSolanaTransaction {
        let message = self.transaction.message.clone();
        let message_bytes = message.serialize();
        let signable_message_hex = hex::encode(&message_bytes);

        // Convert compiled instructions back to SerializableInstruction
        let instructions = message
            .instructions
            .iter()
            .map(|compiled_ix| {
                let ix = SolanaInstruction {
                    program_id: message.account_keys[compiled_ix.program_id_index as usize],
                    accounts: compiled_ix
                        .accounts
                        .iter()
                        .map(|account_idx| {
                            let pubkey = message.account_keys[*account_idx as usize];
                            solana_sdk::instruction::AccountMeta {
                                pubkey,
                                is_signer: message.is_signer(*account_idx as usize),
                                is_writable: message.is_maybe_writable(*account_idx as usize, None),
                            }
                        })
                        .collect(),
                    data: compiled_ix.data.clone(),
                };
                SerializableInstruction::from(&ix)
            })
            .collect();

        UnsignedSolanaTransaction {
            params: self.params.clone(),
            instructions,
            signable_message_hex,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SerializeableVerifierSet {
    pub(crate) signers: BTreeMap<String, u128>,
    pub(crate) nonce: u64,
    pub(crate) threshold: u128,
}

impl From<SerializeableVerifierSet> for VerifierSet {
    fn from(value: SerializeableVerifierSet) -> Self {
        let signers: BTreeMap<PublicKey, u128> = value
            .signers
            .iter()
            .map(|(pk_str, weight)| {
                let pk_bytes: [u8; 33] = hex::decode(pk_str)
                    .expect("Failed to decode public key")
                    .try_into()
                    .expect("Invalid public key length");
                (PublicKey::Secp256k1(pk_bytes), *weight)
            })
            .collect();

        Self {
            signers,
            nonce: value.nonce,
            quorum: value.threshold,
        }
    }
}

/// Uitility verifier set representation that has access to the signing keys
#[derive(Clone, Debug)]
pub(crate) struct SigningVerifierSet {
    /// signers that have access to the given verifier set
    pub(crate) signers: Vec<LocalSigner>,
    /// the nonce for the verifier set
    pub(crate) nonce: u64,
    /// quorum for the verifier set
    pub(crate) quorum: u128,
}

impl SigningVerifierSet {
    /// Create a new `SigningVerifierSet`
    ///
    /// # Panics
    /// if the calculated quorum is larger than u128
    pub(crate) fn new(signers: Vec<LocalSigner>, nonce: u64) -> Self {
        let quorum = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(0, u128::checked_add)
            .expect("no arithmetic overflow");
        Self::new_with_quorum(signers, nonce, quorum)
    }

    /// Create a new `SigningVerifierSet` with a custom quorum
    #[must_use]
    pub(crate) const fn new_with_quorum(
        signers: Vec<LocalSigner>,
        nonce: u64,
        quorum: u128,
    ) -> Self {
        Self {
            signers,
            nonce,
            quorum,
        }
    }

    /// Transform into the verifier set that the gateway expects to operate on
    #[must_use]
    pub(crate) fn verifier_set(&self) -> VerifierSet {
        let signers = self
            .signers
            .iter()
            .map(|x| {
                let pubkey = x.secret.public_key();
                (
                    PublicKey::Secp256k1(
                        pubkey
                            .to_encoded_point(true)
                            .as_bytes()
                            .to_owned()
                            .try_into()
                            .expect("Invalid pubkey derived from secret"),
                    ),
                    x.weight,
                )
            })
            .collect();
        VerifierSet {
            nonce: self.nonce,
            signers,
            quorum: self.quorum,
        }
    }
}

/// Single test signer
#[derive(Clone, Debug)]
pub(crate) struct LocalSigner {
    pub(crate) secret: k256::SecretKey,
    /// associated weight
    pub(crate) weight: u128,
}
