use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use eyre::eyre;
use futures::future::join_all;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::its;

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    Test(TestArgs),
    Verify(VerifyArgs),
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct TestArgs {
    #[clap(long)]
    pub destination_chain: String,

    #[clap(long, value_parser = parse_hex_bytes32)]
    pub token_id: [u8; 32],

    #[clap(long)]
    pub destination_address: String,

    #[clap(long)]
    pub transfer_amount: String,

    #[clap(long)]
    pub gas_value: Option<u64>,

    #[clap(long)]
    pub time: u64,

    #[clap(long, default_value = "10")]
    pub delay: u64,

    #[clap(long, env = "MNEMONIC")]
    pub mnemonic: Option<String>,

    #[clap(long, env = "DERIVE_ACCOUNTS")]
    pub addresses_to_derive: Option<usize>,

    #[clap(long, default_value = "output/load-test.txt")]
    pub output: PathBuf,
}

#[derive(Parser, Debug)]
pub(crate) struct VerifyArgs {
    #[clap(long, default_value = "output/load-test.txt")]
    pub input_file: PathBuf,

    #[clap(long, default_value = "output/load-test-fail.txt")]
    pub fail_output: PathBuf,

    #[clap(long, default_value = "output/load-test-pending.txt")]
    pub pending_output: PathBuf,

    #[clap(long, default_value = "output/load-test-success.txt")]
    pub success_output: PathBuf,

    /// Resume verification starting from this transaction number (1-based).
    /// Use 1 to start from the beginning (default).
    #[clap(long, default_value = "1")]
    pub resume_from: usize,

    #[clap(long, default_value = "100")]
    pub delay: u64,
}

fn parse_hex_bytes32(s: &str) -> eyre::Result<[u8; 32]> {
    let decoded: [u8; 32] = hex::decode(s.trim_start_matches("0x"))?
        .try_into()
        .map_err(|_| eyre!("Invalid hex string length. Expected 32 bytes."))?;
    Ok(decoded)
}

pub(crate) async fn handle_command(command: Commands, config: &Config) -> eyre::Result<()> {
    match command {
        Commands::Test(args) => run_load_test(args, config).await,
        Commands::Verify(args) => verify_transactions(args, config).await,
    }
}

#[allow(clippy::too_many_lines)]
async fn run_load_test(args: TestArgs, config: &Config) -> eyre::Result<()> {
    println!("Starting load test...");
    println!("Destination chain: {}", args.destination_chain);
    println!("Token ID: {}", hex::encode(args.token_id));
    println!("Transfer amount: {}", args.transfer_amount);
    println!("Duration: {} seconds", args.time);
    println!("Delay: {} ms", args.delay);
    println!("Output file: {}", args.output.display());

    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output_file = Arc::new(Mutex::new(
        File::create(&args.output).map_err(|e| eyre!("Failed to create output file: {}", e))?,
    ));

    let keypairs = if let Some(ref mnemonic) = args.mnemonic {
        let count = args
            .addresses_to_derive
            .ok_or_else(|| eyre!("Must specify --addresses-to-derive when using mnemonic"))?;
        if count == 0 {
            return Err(eyre!("--addresses-to-derive must be at least 1"));
        }
        derive_keypairs_from_mnemonic(mnemonic, count)?
    } else {
        return Err(eyre!(
            "Currently only mnemonic-based keypair derivation is supported"
        ));
    };

    println!("Derived {} keypairs for testing", keypairs.len());

    let start_time = Instant::now();
    let duration = Duration::from_secs(args.time);
    let delay_duration = Duration::from_millis(args.delay);

    let tx_count = Arc::new(Mutex::new(0u64));
    let mut pending_tasks = Vec::new();

    let mut keypair_index = 0;

    loop {
        if start_time.elapsed() >= duration {
            break;
        }

        if keypair_index >= keypairs.len() {
            keypair_index = 0;
            tokio::time::sleep(delay_duration).await;
            continue;
        }

        let keypair = Arc::clone(&keypairs[keypair_index]);
        keypair_index += 1;

        let config_clone = config.clone();
        let args_clone = args.clone();
        let output_file_clone = Arc::clone(&output_file);
        let tx_count_clone = Arc::clone(&tx_count);

        let handle = tokio::spawn(async move {
            let transfer_args = (
                args_clone.destination_chain,
                args_clone.token_id,
                args_clone.destination_address,
                args_clone.transfer_amount,
                args_clone.gas_value,
            );

            match execute_transfer(keypair, transfer_args, config_clone) {
                Ok(signature) => {
                    let count = {
                        let mut guard = tx_count_clone.lock().await;
                        *guard += 1;
                        *guard
                    };

                    {
                        let mut file = output_file_clone.lock().await;
                        if let Err(e) = writeln!(file, "{signature}") {
                            eprintln!("Failed to write signature to file: {e}");
                        }
                    }

                    println!("Transaction {count} completed: {signature}");
                }
                Err(e) => {
                    eprintln!("Transaction failed: {e}");
                }
            }
        });

        pending_tasks.push(handle);

        tokio::time::sleep(delay_duration).await;
    }

    let completed_during_test = *tx_count.lock().await;
    let test_duration = start_time.elapsed().as_secs_f64();

    let pending_count = pending_tasks.len();
    println!("Waiting for {pending_count} pending transactions to complete...");

    join_all(pending_tasks).await;

    let final_count = *tx_count.lock().await;
    let total_elapsed = start_time.elapsed().as_secs_f64();

    println!("\n========================================");
    println!("Load test completed!");
    println!("Transactions completed during test window: {completed_during_test}");
    println!("Test window duration: {test_duration:.2} seconds");
    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
    if test_duration > 0.0 {
        let tps = completed_during_test as f64 / test_duration;
        println!("Throughput (during test window): {tps:.2} TPS");
    }
    println!("----------------------------------------");
    println!("Total transactions (including cleanup): {final_count}");
    println!("Total elapsed time: {total_elapsed:.2} seconds");
    println!("Output file: {}", args.output.display());
    println!("========================================\n");

    Ok(())
}

fn execute_transfer(
    keypair: Arc<dyn Signer + Send + Sync>,
    args: (String, [u8; 32], String, String, Option<u64>),
    config: Config,
) -> eyre::Result<Signature> {
    let (destination_chain, token_id, destination_address, transfer_amount, gas_value) = args;

    let source_account = {
        let mint = get_mint_from_token_manager(&token_id, &config)?;
        let token_program = get_token_program_from_mint(&mint, &config)?;
        get_associated_token_address(&keypair.pubkey(), &mint, &token_program)
    };

    let interchain_transfer_args = its::InterchainTransferArgs {
        source_account,
        token_id,
        destination_chain,
        destination_address,
        amount: transfer_amount,
        gas_value: gas_value.unwrap_or(100_000),
        gas_service: None,
        gas_config_account: None,
        timestamp: None,
        authority: Some(keypair.pubkey()),
    };

    let instructions = its::build_instruction(
        &keypair.pubkey(),
        its::Commands::InterchainTransfer(interchain_transfer_args),
        &config,
    )?;

    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());
    let blockhash = rpc_client.get_latest_blockhash()?;

    if let Some(instruction) = instructions.into_iter().next() {
        let message = solana_sdk::message::Message::new_with_blockhash(
            &[instruction],
            Some(&keypair.pubkey()),
            &blockhash,
        );
        let mut transaction = solana_sdk::transaction::Transaction::new_unsigned(message);

        let signers: Vec<&dyn Signer> = vec![keypair.as_ref()];
        transaction.sign(&signers, blockhash);

        let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
        return Ok(signature);
    }

    Err(eyre!("No instructions generated"))
}

#[derive(borsh::BorshDeserialize, Debug)]
struct FlowSlot {
    _flow_limit: Option<u64>,
    _flow_in: u64,
    _flow_out: u64,
    _epoch: u64,
}

#[derive(borsh::BorshDeserialize, Debug)]
struct TokenManager {
    _ty: u8,
    _token_id: [u8; 32],
    token_address: solana_sdk::pubkey::Pubkey,
    _associated_token_account: solana_sdk::pubkey::Pubkey,
    _flow_slot: FlowSlot,
    _bump: u8,
}

fn get_mint_from_token_manager(
    token_id: &[u8; 32],
    config: &Config,
) -> eyre::Result<solana_sdk::pubkey::Pubkey> {
    use borsh::BorshDeserialize as _;

    let rpc_client = RpcClient::new(config.url.clone());
    let (its_root_pda, _) = find_its_root_pda();
    let (token_manager_pda, _) = find_token_manager_pda(&its_root_pda, token_id);
    let account = rpc_client.get_account(&token_manager_pda)?;
    let mut data = &account.data[8..];
    let token_manager = TokenManager::deserialize(&mut data)?;
    Ok(token_manager.token_address)
}

fn get_token_program_from_mint(
    mint: &solana_sdk::pubkey::Pubkey,
    config: &Config,
) -> eyre::Result<solana_sdk::pubkey::Pubkey> {
    let rpc_client = RpcClient::new(config.url.clone());
    let mint_account = rpc_client.get_account(mint)?;
    Ok(mint_account.owner)
}

fn get_associated_token_address(
    wallet_address: &solana_sdk::pubkey::Pubkey,
    token_mint_address: &solana_sdk::pubkey::Pubkey,
    token_program_id: &solana_sdk::pubkey::Pubkey,
) -> solana_sdk::pubkey::Pubkey {
    let associated_token_program_id = spl_associated_token_account_program_id();
    solana_sdk::pubkey::Pubkey::find_program_address(
        &[
            wallet_address.as_ref(),
            token_program_id.as_ref(),
            token_mint_address.as_ref(),
        ],
        &associated_token_program_id,
    )
    .0
}

fn spl_associated_token_account_program_id() -> solana_sdk::pubkey::Pubkey {
    solana_sdk::pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
}

fn find_its_root_pda() -> (solana_sdk::pubkey::Pubkey, u8) {
    solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"interchain-token-service"],
        &solana_axelar_its::id(),
    )
}

fn find_token_manager_pda(
    its_root_pda: &solana_sdk::pubkey::Pubkey,
    token_id: &[u8; 32],
) -> (solana_sdk::pubkey::Pubkey, u8) {
    solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"token-manager", its_root_pda.as_ref(), token_id],
        &solana_axelar_its::id(),
    )
}

async fn verify_transactions(args: VerifyArgs, config: &Config) -> eyre::Result<()> {
    println!("Starting transaction verification...");
    println!("Input file: {}", args.input_file.display());

    let content = std::fs::read_to_string(&args.input_file)
        .map_err(|e| eyre!("Failed to read input file: {}", e))?;

    let mut transactions: Vec<String> = content.lines().map(|s| s.trim().to_owned()).collect();
    transactions.retain(|s| !s.is_empty());

    let total_transactions = transactions.len();

    if args.resume_from > 1 {
        transactions = transactions
            .into_iter()
            .skip(args.resume_from - 1)
            .collect();
        println!(
            "Resuming from transaction {} (remaining: {})",
            args.resume_from,
            transactions.len()
        );
    }

    let stream_flags = if args.resume_from > 1 { "a" } else { "w" };

    for path in [&args.fail_output, &args.pending_output, &args.success_output] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut fail_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(stream_flags == "w")
        .append(stream_flags == "a")
        .open(&args.fail_output)?;

    let mut pending_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(stream_flags == "w")
        .append(stream_flags == "a")
        .open(&args.pending_output)?;

    let mut success_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(stream_flags == "w")
        .append(stream_flags == "a")
        .open(&args.success_output)?;

    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());

    let mut successful = 0;
    let mut failed = 0;
    let mut pending = 0;

    for (index, tx_str) in transactions.iter().enumerate() {
        let current_index = index + args.resume_from - 1;
        println!(
            "Verifying transaction {} of {}: {}",
            current_index + 1,
            total_transactions,
            tx_str
        );

        let signature = match tx_str.parse::<Signature>() {
            Ok(sig) => sig,
            Err(e) => {
                eprintln!("Invalid signature format: {e}");
                writeln!(fail_file, "{tx_str} : invalid signature format")?;
                failed += 1;
                continue;
            }
        };

        match verify_single_transaction(&signature, &rpc_client, config).await {
            VerificationResult::Success => {
                writeln!(success_file, "{tx_str}")?;
                println!("\u{2713} Transaction verified successfully");
                successful += 1;
            }
            VerificationResult::Pending(msg) => {
                writeln!(pending_file, "{tx_str} : {msg}")?;
                println!("\u{29d7} Transaction pending: {msg}");
                pending += 1;
            }
            VerificationResult::Failed(msg) => {
                writeln!(fail_file, "{tx_str} : {msg}")?;
                println!("\u{2717} Transaction failed: {msg}");
                failed += 1;
            }
        }

        tokio::time::sleep(Duration::from_millis(args.delay)).await;
    }

    println!("\n========================================");
    println!("Verification completed!");
    println!("Successful: {successful}");
    println!("Failed: {failed}");
    println!("Pending: {pending}");
    println!("========================================\n");

    Ok(())
}

enum VerificationResult {
    Success,
    Pending(String),
    Failed(String),
}

async fn verify_single_transaction(
    signature: &Signature,
    rpc_client: &RpcClient,
    config: &Config,
) -> VerificationResult {
    match rpc_client.get_signature_status(signature) {
        Ok(Some(status)) => {
            if let Err(e) = status {
                return VerificationResult::Failed(format!("Solana transaction error: {e}"));
            }
        }
        Ok(None) => {
            return VerificationResult::Pending(
                "Solana transaction not found or not finalized".to_owned(),
            );
        }
        Err(e) => {
            return VerificationResult::Failed(format!("Solana RPC error: {e}"));
        }
    }

    if let Err(e) = verify_axelarscan_gmp(signature, config).await {
        return VerificationResult::Failed(format!("Axelarscan verification error: {e}"));
    }

    VerificationResult::Success
}

async fn verify_axelarscan_gmp(signature: &Signature, config: &Config) -> eyre::Result<()> {
    let axelarscan_url = get_axelarscan_api_url(config)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let search_url = format!("{axelarscan_url}/gmp/searchGMP");

    let first_response: serde_json::Value = client
        .post(&search_url)
        .json(&serde_json::json!({
            "txHash": signature.to_string()
        }))
        .send()
        .await?
        .json()
        .await?;

    let message_id = first_response
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("callback"))
        .and_then(|c| c.get("id"))
        .and_then(|id| id.as_str())
        .ok_or_else(|| eyre!("Message ID not found in Axelarscan response"))?;

    let second_response: serde_json::Value = client
        .post(&search_url)
        .json(&serde_json::json!({
            "messageId": message_id
        }))
        .send()
        .await?
        .json()
        .await?;

    let status = second_response
        .get("data")
        .and_then(|d| d.get(0))
        .and_then(|d| d.get("status"))
        .and_then(|s| s.as_str())
        .ok_or_else(|| eyre!("Status not found in Axelarscan response"))?;

    if status != "executed" {
        return Err(eyre!("GMP status is {} (expected executed)", status));
    }

    Ok(())
}

fn get_axelarscan_api_url(config: &Config) -> eyre::Result<String> {
    let chains_info: serde_json::Value =
        crate::utils::read_json_file_from_path(&config.chains_info_file)?;

    let api_url = chains_info
        .get("axelar")
        .and_then(|a| a.get("axelarscanApi"))
        .and_then(|u| u.as_str())
        .ok_or_else(|| eyre!("axelarscanApi not found in chains info"))?;

    Ok(api_url.to_owned())
}

fn derive_keypairs_from_mnemonic(
    mnemonic: &str,
    count: usize,
) -> eyre::Result<Vec<Arc<dyn Signer + Send + Sync>>> {
    use solana_sdk::signature::keypair_from_seed;

    let seed = bip39::Mnemonic::parse(mnemonic)
        .map_err(|e| eyre!("Invalid mnemonic: {}", e))?
        .to_seed("");

    let mut keypairs: Vec<Arc<dyn Signer + Send + Sync>> = Vec::with_capacity(count);

    for i in 0..count {
        let derivation_path = format!("m/44'/501'/{i}'");
        let derived_key = derive_key_from_seed(&seed, &derivation_path)?;
        let keypair = keypair_from_seed(&derived_key[..32])
            .map_err(|e| eyre!("Failed to create keypair: {}", e))?;
        keypairs.push(Arc::new(keypair));
    }

    Ok(keypairs)
}

#[allow(clippy::big_endian_bytes, clippy::missing_asserts_for_indexing)]
fn derive_key_from_seed(seed: &[u8], path: &str) -> eyre::Result<[u8; 64]> {
    use hmac::Hmac;
    use hmac::Mac;
    use sha2::Sha512;

    let mut hmac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed")
        .map_err(|e| eyre!("HMAC initialization failed: {}", e))?;
    hmac.update(seed);
    let result = hmac.finalize();
    let bytes = result.into_bytes();

    if bytes.len() <= 63 {
        return Err(eyre!("HMAC output too short"));
    }
    let mut key = [0u8; 64];
    key[..32].copy_from_slice(&bytes[..32]);
    key[32..].copy_from_slice(&bytes[32..64]);

    let parts: Vec<&str> = path.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 && *part == "m" {
            continue;
        }

        let hardened = part.ends_with('\'');
        let index_str = part.trim_end_matches('\'');
        let index: u32 = index_str
            .parse()
            .map_err(|_| eyre!("Invalid derivation path index: {}", part))?;

        let child_index = if hardened { 0x8000_0000 | index } else { index };

        let mut data = Vec::with_capacity(37);
        data.push(0);
        data.extend_from_slice(&key[..32]);
        data.extend_from_slice(&child_index.to_be_bytes());

        let mut hmac = Hmac::<Sha512>::new_from_slice(&key[32..64])
            .map_err(|e| eyre!("HMAC initialization failed: {}", e))?;
        hmac.update(&data);
        let result = hmac.finalize();
        let bytes = result.into_bytes();

        if bytes.len() <= 63 {
            return Err(eyre!("HMAC output too short"));
        }
        key[..32].copy_from_slice(&bytes[..32]);
        key[32..].copy_from_slice(&bytes[32..64]);
    }

    Ok(key)
}
