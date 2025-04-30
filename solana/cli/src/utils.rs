use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::LazyLock;

use crate::config::Config;
use crate::error::{AppError, Result};
use crate::types::{
    NetworkType, PartialSignature, SignedSolanaTransaction, UnsignedSolanaTransaction,
};

const DEVNET_AMPLIFIER_CONFIG: &'static str = include_str!("../devnet-amplifier.json");
const TESTNET_AMPLIFIER_CONFIG: &'static str = include_str!("../testnet.json");
const MAINNET_AMPLIFIER_CONFIG: &'static str = include_str!("../mainnet.json");

pub(crate) const ADDRESS_KEY: &str = "address";
pub(crate) const AXELAR_KEY: &str = "axelar";
pub(crate) const CHAINS_KEY: &str = "chains";
pub(crate) const CHAIN_TYPE_KEY: &str = "chainType";
pub(crate) const CONTRACTS_KEY: &str = "contracts";
pub(crate) const DOMAIN_SEPARATOR_KEY: &str = "domainSeparator";
pub(crate) const GAS_CONFIG_ACCOUNT: &str = "configAccount";
pub(crate) const GAS_SERVICE_KEY: &str = "AxelarGasService";
pub(crate) const GATEWAY_KEY: &str = "AxelarGateway";
pub(crate) const GRPC_KEY: &str = "grpc";
pub(crate) const ITS_KEY: &str = "InterchainTokenService";
pub(crate) const MULTISIG_PROVER_KEY: &str = "MultisigProver";

static MAINNET_INFO: LazyLock<serde_json::Value> =
    LazyLock::new(|| serde_json::from_str(MAINNET_AMPLIFIER_CONFIG).unwrap());

static TESTNET_INFO: LazyLock<serde_json::Value> =
    LazyLock::new(|| serde_json::from_str(TESTNET_AMPLIFIER_CONFIG).unwrap());

static DEVNET_INFO: LazyLock<serde_json::Value> =
    LazyLock::new(|| serde_json::from_str(DEVNET_AMPLIFIER_CONFIG).unwrap());

pub(crate) fn chains_info(network: NetworkType) -> &'static serde_json::Value {
    match network {
        NetworkType::Mainnet => &*MAINNET_INFO,
        NetworkType::Testnet => &*TESTNET_INFO,
        NetworkType::Devnet => &*DEVNET_INFO,
        NetworkType::Localnet => panic!(
            "Cannot automatically load chains info for Localnet. \
             Please pass the required arguments to the CLI."
        ),
    }
}

pub(crate) fn save_chains_info(network: NetworkType, info: serde_json::Value, output_dir: &Path) {
    let path = match network {
        NetworkType::Mainnet => "mainnet.json",
        NetworkType::Testnet => "testnet.json",
        NetworkType::Devnet => "devnet-amplifier.json",
        NetworkType::Localnet => panic!("Cannot save chains info for Localnet."),
    };

    let file = std::fs::File::create(output_dir.join(path)).expect("Unable to create file");
    serde_json::to_writer_pretty(file, &info).expect("Unable to write data");
}

pub(crate) fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let file = File::open(path).map_err(|e| AppError::IoError(e))?;
    let reader = std::io::BufReader::new(file);
    serde_json::from_reader(reader).map_err(|e| AppError::JsonError(e))
}

pub(crate) fn write_json_file<T: Serialize>(data: &T, path: &Path) -> Result<()> {
    let file = File::create(path).map_err(|e| AppError::IoError(e))?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, data).map_err(|e| AppError::JsonError(e))
}

pub(crate) fn load_unsigned_solana_transaction(path: &Path) -> Result<UnsignedSolanaTransaction> {
    read_json_file(path)
}

pub(crate) fn save_unsigned_solana_transaction(
    tx: &UnsignedSolanaTransaction,
    path: &Path,
) -> Result<()> {
    write_json_file(tx, path)
}

pub(crate) fn load_partial_signature(path: &Path) -> Result<PartialSignature> {
    read_json_file(path)
}

pub(crate) fn save_partial_signature(sig: &PartialSignature, path: &Path) -> Result<()> {
    write_json_file(sig, path)
}

pub(crate) fn load_signed_solana_transaction(path: &Path) -> Result<SignedSolanaTransaction> {
    read_json_file(path)
}

pub(crate) fn save_signed_solana_transaction(
    tx: &SignedSolanaTransaction,
    path: &Path,
) -> Result<()> {
    write_json_file(tx, path)
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
    config: &Config,
    destination_chain: &str,
    destination_address: String,
) -> eyre::Result<Vec<u8>> {
    let chain_type = String::deserialize(
        &chains_info(config.network_type)[CHAINS_KEY][destination_chain][CHAIN_TYPE_KEY],
    )?;

    match chain_type.to_lowercase().as_str() {
        "stellar" => Ok(destination_address.into_bytes()),
        "svm" => Ok(Pubkey::from_str(&destination_address)?.to_bytes().to_vec()),
        _ => Ok(hex::decode(destination_address)?),
    }
}
