use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use clap::ArgMatches;
use eyre::eyre;
use k256::elliptic_curve::FieldBytes;
use k256::pkcs8::DecodePrivateKey;
use k256::{Secp256k1, SecretKey};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::rpc_client::RpcClient;
use solana_sdk::account_utils::StateMut;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::Instruction;
use solana_sdk::nonce::state::Versions;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;

use crate::config::Config;
use crate::types::{
    NetworkType, PartialSignature, SignedSolanaTransaction, UnsignedSolanaTransaction,
};
pub(crate) use solana_sdk::instruction::AccountMeta;

pub(crate) const DEFAULT_COMPUTE_UNITS: u32 = 1_400_000; // Maximum allowed is 1.4M compute units
pub(crate) const DEFAULT_PRIORITY_FEE: u64 = 10_000; // 10,000 micro-lamports per compute unit

pub(crate) fn create_compute_budget_instructions(
    compute_units: u32,
    priority_fee: u64,
) -> Vec<Instruction> {
    vec![
        ComputeBudgetInstruction::set_compute_unit_limit(compute_units),
        ComputeBudgetInstruction::set_compute_unit_price(priority_fee),
    ]
}

pub(crate) const ADDRESS_KEY: &str = "address";
pub(crate) const AXELAR_KEY: &str = "axelar";
pub(crate) const CHAINS_KEY: &str = "chains";
pub(crate) const CHAIN_TYPE_KEY: &str = "chainType";
pub(crate) const CONFIG_ACCOUNT_KEY: &str = "configAccount";
pub(crate) const CONNECTION_TYPE_KEY: &str = "connectionType";
pub(crate) const CONTRACTS_KEY: &str = "contracts";
pub(crate) const DOMAIN_SEPARATOR_KEY: &str = "domainSeparator";
pub(crate) const GAS_SERVICE_KEY: &str = "AxelarGasService";
pub(crate) const GATEWAY_KEY: &str = "AxelarGateway";
pub(crate) const GOVERNANCE_ADDRESS_KEY: &str = "governanceAddress";
pub(crate) const GOVERNANCE_CHAIN_KEY: &str = "governanceChain";
pub(crate) const GOVERNANCE_KEY: &str = "InterchainGovernance";
pub(crate) const MULTICALL_KEY: &str = "Multicall";
pub(crate) const GRPC_KEY: &str = "grpc";
pub(crate) const ITS_KEY: &str = "InterchainTokenService";
pub(crate) const MINIMUM_PROPOSAL_ETA_DELAY_KEY: &str = "minimumTimeDelay";
pub(crate) const MINIMUM_ROTATION_DELAY_KEY: &str = "minimumRotationDelay";
pub(crate) const MULTISIG_PROVER_KEY: &str = "SolanaMultisigProver";
pub(crate) const OPERATOR_KEY: &str = "operator";
pub(crate) const PREVIOUS_SIGNERS_RETENTION_KEY: &str = "previousSignersRetention";
pub(crate) const UPGRADE_AUTHORITY_KEY: &str = "upgradeAuthority";

pub(crate) fn read_json_file<T: DeserializeOwned>(file: &File) -> eyre::Result<T> {
    let reader = std::io::BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

pub(crate) fn write_json_file<T: Serialize>(data: &T, file: &File) -> eyre::Result<()> {
    let writer = std::io::BufWriter::new(file);
    Ok(serde_json::to_writer_pretty(writer, data)?)
}

pub(crate) fn read_json_file_from_path<T: DeserializeOwned>(path: &Path) -> eyre::Result<T> {
    let file = File::open(path)?;
    read_json_file(&file)
}

pub(crate) fn write_json_to_file_path<T: Serialize>(data: &T, path: &Path) -> eyre::Result<()> {
    let file = File::create(path)?;
    write_json_file(data, &file)
}

pub(crate) fn load_unsigned_solana_transaction(
    path: &Path,
) -> eyre::Result<UnsignedSolanaTransaction> {
    read_json_file_from_path(path)
}

pub(crate) fn save_unsigned_solana_transaction(
    tx: &UnsignedSolanaTransaction,
    path: &Path,
) -> eyre::Result<()> {
    write_json_to_file_path(tx, path)
}

pub(crate) fn load_partial_signature(path: &Path) -> eyre::Result<PartialSignature> {
    read_json_file_from_path(path)
}

pub(crate) fn save_partial_signature(sig: &PartialSignature, path: &Path) -> eyre::Result<()> {
    write_json_to_file_path(sig, path)
}

pub(crate) fn load_signed_solana_transaction(path: &Path) -> eyre::Result<SignedSolanaTransaction> {
    match read_json_file_from_path::<SignedSolanaTransaction>(path) {
        Ok(signed_tx) => Ok(signed_tx),
        Err(err) => match read_json_file_from_path::<UnsignedSolanaTransaction>(path) {
            Ok(unsigned_tx) => {
                println!(
                    "Warning: Found unsigned transaction, converting to signed format without signatures"
                );
                Ok(SignedSolanaTransaction {
                    unsigned_tx_data: unsigned_tx,
                    signatures: Vec::new(),
                })
            }
            Err(_) => Err(err),
        },
    }
}

pub(crate) fn save_signed_solana_transaction(
    tx: &SignedSolanaTransaction,
    path: &Path,
) -> eyre::Result<()> {
    write_json_to_file_path(tx, path)
}

pub(crate) fn decode_its_destination(
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
pub(crate) fn parse_account_meta_string(s: &str) -> eyre::Result<AccountMeta> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        eyre::bail!("Invalid AccountMeta format: '{s}'. Expected 'pubkey:is_signer:is_writable'");
    }

    let pubkey = Pubkey::from_str(parts[0])?;
    let is_signer = bool::from_str(parts[1]).map_err(|_| {
        eyre!(
            "Invalid is_signer value: '{}'. Expected 'true' or 'false'",
            parts[1]
        )
    })?;
    let is_writable = bool::from_str(parts[2]).map_err(|_| {
        eyre!(
            "Invalid is_writable value: '{}'. Expected 'true' or 'false'",
            parts[2]
        )
    })?;

    Ok(if is_writable {
        AccountMeta::new(pubkey, is_signer)
    } else {
        AccountMeta::new_readonly(pubkey, is_signer)
    })
}

pub(crate) fn print_transaction_result(
    config: &Config,
    result: eyre::Result<Signature>,
) -> eyre::Result<()> {
    match result {
        Ok(tx_signature) => {
            println!("------------------------------------------");
            println!("\u{2705} Solana Transaction successfully broadcast and confirmed!");
            println!("   Transaction Signature (ID): {tx_signature}");
            println!("   RPC Endpoint: {}", config.url);
            let explorer_base_url = "https://explorer.solana.com/tx/";
            let cluster_param = match config.network_type {
                NetworkType::Local => "?cluster=custom",
                NetworkType::Devnet => "?cluster=devnet",
                NetworkType::Testnet => "?cluster=testnet",
                NetworkType::Mainnet => "",
            };
            println!("   Explorer Link: {explorer_base_url}{tx_signature}{cluster_param}");
            println!("------------------------------------------");

            Ok(())
        }
        Err(e) => {
            eprintln!("------------------------------------------");
            eprintln!("\u{274c} Solana Transaction broadcast failed.");
            eprintln!("------------------------------------------");

            Err(e)
        }
    }
}

pub(crate) fn domain_separator(
    chains_info: &serde_json::Value,
    network_type: NetworkType,
    chain: &str,
) -> eyre::Result<[u8; 32]> {
    if network_type == NetworkType::Local {
        return Ok([0; 32]);
    }

    let from_multisig_prover = String::deserialize(
        &chains_info[AXELAR_KEY][CONTRACTS_KEY][MULTISIG_PROVER_KEY][chain]
            [DOMAIN_SEPARATOR_KEY],
    )?;

    let domain_separator: [u8; 32] = hex::decode(from_multisig_prover.trim_start_matches("0x"))?
        .try_into()
        .expect("invalid domain separator");

    Ok(domain_separator)
}

pub(crate) fn parse_secret_key(raw: &str) -> eyre::Result<SecretKey> {
    if Path::new(raw).exists() {
        let bytes = std::fs::read(raw)?;
        return secret_from_bytes(&bytes)
            .or_else(|| secret_from_str(std::str::from_utf8(&bytes).ok()?))
            .ok_or_else(|| eyre!("unrecognised key format in file"));
    }

    secret_from_str(raw).ok_or_else(|| eyre!("unrecognised key format"))
}

pub(crate) fn fetch_latest_blockhash(rpc_url: &str) -> eyre::Result<Hash> {
    let rpc_client = RpcClient::new(rpc_url.to_owned());
    Ok(rpc_client.get_latest_blockhash()?)
}

pub(crate) fn fetch_nonce_data_and_verify(
    rpc_url: &str,
    nonce_account_pubkey: &Pubkey,
    expected_nonce_authority: &Pubkey,
) -> eyre::Result<Hash> {
    let rpc_client = RpcClient::new(rpc_url.to_owned());
    let nonce_account = rpc_client.get_account(nonce_account_pubkey)?;

    if !solana_sdk::system_program::check_id(&nonce_account.owner) {
        eyre::bail!(
            "Nonce account {} is not owned by the system program ({}), owner is {}",
            nonce_account_pubkey,
            solana_sdk::system_program::id(),
            nonce_account.owner
        );
    }

    let nonce_state: solana_sdk::nonce::state::State = StateMut::<Versions>::state(&nonce_account)
        .map_err(|_| eyre!("Failed to deserialize nonce account {nonce_account_pubkey}"))?
        .into();

    match nonce_state {
        solana_sdk::nonce::state::State::Initialized(data) => {
            println!("Nonce account is initialized.");
            println!(" -> Stored Nonce (Blockhash): {}", data.blockhash());
            println!(" -> Authority: {}", data.authority);

            if data.authority != *expected_nonce_authority {
                return Err(eyre!(
                    "Nonce account authority mismatch: expected {}, found {}",
                    expected_nonce_authority,
                    data.authority
                ));
            }

            Ok(data.blockhash())
        }
        solana_sdk::nonce::state::State::Uninitialized => Err(eyre!(
            "Nonce account {nonce_account_pubkey} is uninitialized"
        )),
    }
}

fn secret_from_bytes(b: &[u8]) -> Option<SecretKey> {
    SecretKey::from_pkcs8_der(b)
        .ok()
        .or_else(|| SecretKey::from_sec1_der(b).ok())
        .or_else(|| (b.len() == 32).then(|| SecretKey::from_bytes(b.into()).ok())?)
}

fn secret_from_str(s: &str) -> Option<SecretKey> {
    let s = s.trim();

    // PEM (SEC1 or PKCS8)
    if s.starts_with("-----BEGIN") {
        return SecretKey::from_pkcs8_pem(s)
            .ok()
            .or_else(|| SecretKey::from_sec1_pem(s).ok());
    }

    // raw hex
    if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
        let bytes = hex::decode(s).ok()?;
        return SecretKey::from_bytes(FieldBytes::<Secp256k1>::from_slice(&bytes)).ok();
    }

    None
}

pub(crate) fn serialized_transactions_filename_from_arg_matches(matches: &ArgMatches) -> String {
    let mut chain = Vec::<String>::new();
    let mut m = matches;
    while let Some((name, sub)) = m.subcommand() {
        chain.push(name.to_owned());
        m = sub;
    }

    chain.into_iter().skip(1).collect::<Vec<_>>().join("-")
}

pub(crate) fn try_infer_program_id_from_env(
    env: &Value,
    chain: &str,
    program_key: &str,
) -> eyre::Result<Pubkey> {
    let id = Pubkey::from_str(&String::deserialize(
        &env[CHAINS_KEY][chain][CONTRACTS_KEY][program_key][ADDRESS_KEY],
    )?)
    .map_err(|_| {
        eyre!(
            "Could not get the program id ({}) from the chains info JSON file. Is it already deployed?", program_key
        )
    })?;

    Ok(id)
}

pub(crate) fn safe_token_conversion(human_amount: f64, decimals: u8) -> eyre::Result<u64> {
    // Validate input
    if human_amount < 0.0 {
        return Err(eyre::eyre!("Token amount cannot be negative: {}", human_amount));
    }
    
    if decimals > 19 {
        return Err(eyre::eyre!("Too many decimals: {} (maximum 19)", decimals));
    }
    
    // Handle zero amount
    if human_amount == 0.0 {
        return Ok(0);
    }
    
    // Convert to string to avoid floating-point precision issues
    let amount_str = format!("{:.19}", human_amount);
    let parts: Vec<&str> = amount_str.split('.').collect();
    
    let integer_part = parts[0].parse::<u64>()
        .map_err(|_| eyre::eyre!("Invalid integer part: {}", parts[0]))?;
    
    let fractional_part = if parts.len() > 1 {
        let frac_str = parts[1];
        let frac_len = frac_str.len().min(decimals as usize);
        let truncated_frac = &frac_str[..frac_len];
        
        // Pad with zeros if necessary
        let padded_frac = format!("{:0<width$}", truncated_frac, width = decimals as usize);
        padded_frac.parse::<u64>()
            .map_err(|_| eyre::eyre!("Invalid fractional part: {}", truncated_frac))?
    } else {
        0
    };
    
    // Calculate the multiplier as a u64 to avoid floating-point issues
    let multiplier = 10_u64.pow(decimals as u32);
    
    // Check for overflow before multiplication
    if integer_part > u64::MAX / multiplier {
        return Err(eyre::eyre!(
            "Amount too large: {} * 10^{} would overflow u64::MAX ({})",
            human_amount,
            decimals,
            u64::MAX
        ));
    }
    
    let integer_contribution = integer_part * multiplier;
    
    // Check if adding fractional part would overflow
    if integer_contribution > u64::MAX - fractional_part {
        return Err(eyre::eyre!(
            "Amount too large: {} * 10^{} would overflow u64::MAX ({})",
            human_amount,
            decimals,
            u64::MAX
        ));
    }
    
    Ok(integer_contribution + fractional_part)
}

/// Parses a decimal string and converts it to raw units with specified decimals.
/// This function avoids floating-point precision issues by working directly with strings.
/// 
/// # Arguments
/// * `s` - The decimal string to parse (e.g., "123.55", "1000", "0.001")
/// * `decimals` - The number of decimal places for the token
///
/// # Returns
/// * `Ok(u64)` - The raw amount in smallest units
/// * `Err(eyre::Error)` - If parsing fails
///
/// # Example
/// ```
/// let raw_amount = parse_decimal_string_to_raw_units("123.55", 9)?; // Returns 123550000000
/// ```
pub(crate) fn parse_decimal_string_to_raw_units(s: &str, decimals: u8) -> eyre::Result<u64> {
    // Validate input
    if s.is_empty() {
        return Err(eyre::eyre!("Amount cannot be empty"));
    }
    
    if decimals > 19 {
        return Err(eyre::eyre!("Too many decimals: {} (maximum 19)", decimals));
    }
    
    // Handle negative amounts
    if s.starts_with('-') {
        return Err(eyre::eyre!("Amount cannot be negative: {}", s));
    }
    
    // Split by decimal point
    let parts: Vec<&str> = s.split('.').collect();
    
    if parts.len() > 2 {
        return Err(eyre::eyre!("Invalid decimal format: {}", s));
    }
    
    let integer_part = parts[0];
    let fractional_part = if parts.len() > 1 { parts[1] } else { "" };
    
    // Parse integer part
    let integer_value = if integer_part.is_empty() {
        0
    } else {
        integer_part.parse::<u64>()
            .map_err(|_| eyre::eyre!("Invalid integer part: {}", integer_part))?
    };
    
    // Parse fractional part
    let fractional_value = if fractional_part.is_empty() {
        0
    } else {
        // Truncate fractional part to fit within decimals
        let truncated_frac = if fractional_part.len() > decimals as usize {
            &fractional_part[..decimals as usize]
        } else {
            fractional_part
        };
        
        // Pad with zeros if necessary
        let padded_frac = format!("{:0<width$}", truncated_frac, width = decimals as usize);
        padded_frac.parse::<u64>()
            .map_err(|_| eyre::eyre!("Invalid fractional part: {}", truncated_frac))?
    };
    
    // Calculate the multiplier as a u64 to avoid floating-point issues
    let multiplier = 10_u64.pow(decimals as u32);
    
    // Check for overflow before multiplication
    if integer_value > u64::MAX / multiplier {
        return Err(eyre::eyre!(
            "Amount too large: {} * 10^{} would overflow u64::MAX ({})",
            s,
            decimals,
            u64::MAX
        ));
    }
    
    let integer_contribution = integer_value * multiplier;
    
    // Check if adding fractional part would overflow
    if integer_contribution > u64::MAX - fractional_value {
        return Err(eyre::eyre!(
            "Amount too large: {} * 10^{} would overflow u64::MAX ({})",
            s,
            decimals,
            u64::MAX
        ));
    }
    
    Ok(integer_contribution + fractional_value)
}
