use crate::error::{AppError, Result};
use clap::ArgEnum;
use serde::{Deserialize, Serialize};
use solana_sdk::{instruction::Instruction as SolanaInstruction, pubkey::Pubkey};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(ArgEnum, Debug, Copy, Clone, PartialEq, Eq)]
pub enum NetworkType {
    Mainnet,
    Testnet,
    Devnet,
    Localnet,
}

impl FromStr for NetworkType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self> {
        s.contains("mainnet")
            .then_some(NetworkType::Mainnet)
            .or_else(|| s.contains("testnet").then_some(NetworkType::Testnet))
            .or_else(|| s.contains("devnet").then_some(NetworkType::Devnet))
            .or_else(|| s.contains("local").then_some(NetworkType::Localnet))
            .ok_or_else(|| AppError::InvalidNetworkType(s.to_string()))
    }
}

pub struct ChainsInfoFile(pub String);
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

pub struct ChainNameOnAxelar(pub String);

impl From<NetworkType> for ChainNameOnAxelar {
    fn from(value: NetworkType) -> Self {
        match value {
            NetworkType::Mainnet => Self("solana-mainnet".to_owned()),
            NetworkType::Testnet => Self("solana-testnet".to_owned()),
            NetworkType::Devnet => Self("solana-devnet".to_owned()),
            NetworkType::Localnet => Self("solana-localnet".to_owned()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SolanaTransactionParams {
    pub fee_payer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_blockhash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce_account: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce_authority: Option<String>,
    pub blockhash_for_message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableInstruction {
    pub program_id: String,
    pub accounts: Vec<SerializableAccountMeta>,
    #[serde(with = "hex::serde")]
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SerializableAccountMeta {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl TryFrom<&SerializableInstruction> for SolanaInstruction {
    type Error = AppError;

    fn try_from(si: &SerializableInstruction) -> Result<Self> {
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
            .collect::<Result<Vec<_>>>()?;

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
pub struct UnsignedSolanaTransaction {
    pub params: SolanaTransactionParams,
    pub instructions: Vec<SerializableInstruction>,
    pub signable_message_hex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PartialSignature {
    pub signer_pubkey: String,
    pub signature: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignedSolanaTransaction {
    pub unsigned_tx_data: UnsignedSolanaTransaction,
    pub signatures: Vec<PartialSignature>,
}

#[derive(Debug, Clone)]
pub struct SendArgs {
    pub fee_payer: Pubkey,
    pub signers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GenerateArgs {
    pub fee_payer: Pubkey,
    pub nonce_account: Option<Pubkey>,
    pub nonce_authority: Option<Pubkey>,
    pub recent_blockhash: Option<String>,
    pub output_file: String,
}

#[derive(Debug, Clone)]
pub struct SignArgs {
    pub unsigned_tx_path: PathBuf,
    pub signer_key: String,
    pub output_signature_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CombineArgs {
    pub unsigned_tx_path: PathBuf,
    pub signature_paths: Vec<PathBuf>,
    pub output_signed_tx_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct BroadcastArgs {
    pub signed_tx_path: PathBuf,
}
