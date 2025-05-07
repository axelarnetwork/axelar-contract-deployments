use axelar_solana_encoding::types::pubkey::PublicKey;
use axelar_solana_encoding::types::verifier_set::VerifierSet;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{
    NetworkType, PartialSignature, SignedSolanaTransaction, UnsignedSolanaTransaction,
};
pub(crate) use solana_sdk::instruction::AccountMeta;

pub(crate) const ADDRESS_KEY: &str = "address";
pub(crate) const AXELAR_KEY: &str = "axelar";
pub(crate) const CHAINS_KEY: &str = "chains";
pub(crate) const CHAIN_TYPE_KEY: &str = "chainType";
pub(crate) const CONTRACTS_KEY: &str = "contracts";
pub(crate) const DOMAIN_SEPARATOR_KEY: &str = "domainSeparator";
pub(crate) const CONFIG_ACCOUNT_KEY: &str = "configAccount";
pub(crate) const GAS_SERVICE_KEY: &str = "AxelarGasService";
pub(crate) const GATEWAY_KEY: &str = "AxelarGateway";
pub(crate) const GRPC_KEY: &str = "grpc";
pub(crate) const ITS_KEY: &str = "InterchainTokenService";
pub(crate) const MULTISIG_PROVER_KEY: &str = "MultisigProver";
pub(crate) const UPGRADE_AUTHORITY_KEY: &str = "upgradeAuthority";
pub(crate) const OPERATOR_KEY: &str = "operator";
pub(crate) const MINIMUM_ROTATION_DELAY_KEY: &str = "minimumRotationDelay";
pub(crate) const PREVIOUS_SIGNERS_RETENTION_KEY: &str = "previousSignersRetention";
pub(crate) const GOVERNANCE_KEY: &str = "InterchainGovernance";
pub(crate) const MINIMUM_PROPOSAL_ETA_DELAY_KEY: &str = "minimumTimeDelay";
pub(crate) const GOVERNANCE_CHAIN_KEY: &str = "governanceChain";
pub(crate) const GOVERNANCE_ADDRESS_KEY: &str = "governanceAddress";

pub(crate) fn read_json_file<T: DeserializeOwned>(file: &File) -> Result<T> {
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| AppError::JsonError(e))
}

pub(crate) fn write_json_file<T: Serialize>(data: &T, file: &File) -> Result<()> {
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data).map_err(|e| AppError::JsonError(e))
}

pub(crate) fn read_json_file_from_path<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).map_err(|e| AppError::IoError(e))?;
    read_json_file(&file)
}

pub(crate) fn write_json_to_file_path<T: Serialize>(data: &T, path: &Path) -> Result<()> {
    let file = File::create(path).map_err(|e| AppError::IoError(e))?;
    write_json_file(data, &file)
}

pub(crate) fn load_unsigned_solana_transaction(path: &Path) -> Result<UnsignedSolanaTransaction> {
    read_json_file_from_path(path)
}

pub(crate) fn save_unsigned_solana_transaction(
    tx: &UnsignedSolanaTransaction,
    path: &Path,
) -> Result<()> {
    write_json_to_file_path(tx, path)
}

pub(crate) fn load_partial_signature(path: &Path) -> Result<PartialSignature> {
    read_json_file_from_path(path)
}

pub(crate) fn save_partial_signature(sig: &PartialSignature, path: &Path) -> Result<()> {
    write_json_to_file_path(sig, path)
}

pub(crate) fn load_signed_solana_transaction(path: &Path) -> Result<SignedSolanaTransaction> {
    read_json_file_from_path(path)
}

pub(crate) fn save_signed_solana_transaction(
    tx: &SignedSolanaTransaction,
    path: &Path,
) -> Result<()> {
    write_json_to_file_path(tx, path)
}

pub(crate) fn create_offline_bundle(
    bundle_name: &str,
    output_dir: &Path,
    files_to_include: &[(&str, &Path)],
) -> Result<PathBuf> {
    let target_path = output_dir.join(format!("{}.tar.gz", bundle_name));
    let tar_gz_file = File::create(&target_path).unwrap();
    let gz_encoder = flate2::write::GzEncoder::new(tar_gz_file, flate2::Compression::default());
    let mut tar_builder = tar::Builder::new(gz_encoder);
    tar_builder.follow_symlinks(true);

    for (name_in_archive, path_on_disk) in files_to_include {
        if !path_on_disk.exists() {
            return Err(AppError::PackagingError(format!(
                "File specified for packaging not found: {}",
                path_on_disk.display()
            )));
        }
        if path_on_disk.is_file() {
            println!(
                "Adding file to bundle: {} (from {})",
                name_in_archive,
                path_on_disk.display()
            );
            tar_builder
                .append_path_with_name(path_on_disk, name_in_archive)
                .unwrap();
        } else if path_on_disk.is_dir() {
            println!(
                "Adding directory to bundle: {} (from {})",
                name_in_archive,
                path_on_disk.display()
            );
            tar_builder
                .append_dir_all(name_in_archive, path_on_disk)
                .unwrap();
        } else {
            return Err(AppError::PackagingError(format!(
                "Path specified for packaging is not a file or directory: {}",
                path_on_disk.display()
            )));
        }
    }

    let gz_encoder = tar_builder.into_inner().unwrap();
    gz_encoder.finish().unwrap();

    Ok(target_path)
}

pub(crate) fn encode_its_destination(
    chains_info: &serde_json::Value,
    destination_chain: &str,
    destination_address: String,
) -> eyre::Result<Vec<u8>> {
    let chain_type =
        String::deserialize(&chains_info[CHAINS_KEY][destination_chain][CHAIN_TYPE_KEY])?;

    match chain_type.to_lowercase().as_str() {
        "stellar" => Ok(destination_address.into_bytes()),
        "svm" => Ok(Pubkey::from_str(&destination_address)?.to_bytes().to_vec()),
        _ => Ok(hex::decode(destination_address)?),
    }
}

/// Parses a string representation of an AccountMeta.
/// Format: "pubkey:is_signer:is_writable" (e.g., "SomePubkey...:false:true")
pub fn parse_account_meta_string(s: &str) -> Result<AccountMeta> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(AppError::InvalidInput(format!(
            "Invalid AccountMeta format: '{}'. Expected 'pubkey:is_signer:is_writable'",
            s
        )));
    }

    let pubkey = Pubkey::from_str(parts[0])?;
    let is_signer = bool::from_str(parts[1]).map_err(|_| {
        AppError::InvalidInput(format!(
            "Invalid is_signer value: '{}'. Expected 'true' or 'false'",
            parts[1]
        ))
    })?;
    let is_writable = bool::from_str(parts[2]).map_err(|_| {
        AppError::InvalidInput(format!(
            "Invalid is_writable value: '{}'. Expected 'true' or 'false'",
            parts[2]
        ))
    })?;

    Ok(if is_writable {
        AccountMeta::new(pubkey, is_signer)
    } else {
        AccountMeta::new_readonly(pubkey, is_signer)
    })
}

pub(crate) fn print_transaction_result(config: &Config, result: Result<Signature>) -> Result<()> {
    match result {
        Ok(tx_signature) => {
            println!("------------------------------------------");
            println!("✅ Solana Transaction successfully broadcast and confirmed!");
            println!("   Transaction Signature (ID): {}", tx_signature);
            println!("   RPC Endpoint: {}", config.url);
            let explorer_base_url = "https://explorer.solana.com/tx/";
            let cluster_param = match config.network_type {
                NetworkType::Mainnet => "",
                NetworkType::Testnet => "?cluster=testnet",
                NetworkType::Devnet => "?cluster=devnet",
                NetworkType::Localnet => "?cluster=custom",
            };
            println!(
                "   Explorer Link: {}{}{}",
                explorer_base_url, tx_signature, cluster_param
            );
            println!("------------------------------------------");

            Ok(())
        }
        Err(e) => {
            eprintln!("------------------------------------------");
            eprintln!("❌ Solana Transaction broadcast failed.");
            eprintln!("------------------------------------------");

            Err(e.into())
        }
    }
}

/// Uitility verifier set representation that has access to the signing keys
#[derive(Clone, Debug)]
pub struct SigningVerifierSet {
    /// signers that have access to the given verifier set
    pub signers: Vec<TestSigner>,
    /// the nonce for the verifier set
    pub nonce: u64,
    /// quorum for the verifier set
    pub quorum: u128,
}

impl SigningVerifierSet {
    /// Create a new `SigningVerifierSet`
    ///
    /// # Panics
    /// if the calculated quorum is larger than u128
    pub fn new(signers: Vec<TestSigner>, nonce: u64) -> Self {
        let quorum = signers
            .iter()
            .map(|signer| signer.weight)
            .try_fold(0, u128::checked_add)
            .expect("no arithmetic overflow");
        Self::new_with_quorum(signers, nonce, quorum)
    }

    /// Create a new `SigningVerifierSet` with a custom quorum
    #[must_use]
    pub const fn new_with_quorum(signers: Vec<TestSigner>, nonce: u64, quorum: u128) -> Self {
        Self {
            signers,
            nonce,
            quorum,
        }
    }

    /// Transform into the verifier set that the gateway expects to operate on
    #[must_use]
    pub fn verifier_set(&self) -> VerifierSet {
        let signers = self
            .signers
            .iter()
            .map(|x| {
                let pubkey = libsecp256k1::PublicKey::from_secret_key(&x.inner);
                (
                    PublicKey::Secp256k1(pubkey.serialize_compressed()),
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
pub struct TestSigner {
    pub inner: libsecp256k1::SecretKey,
    /// associated weight
    pub weight: u128,
}
