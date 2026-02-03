//! Transaction verification logic for load testing.

use std::io::Write;
use std::time::Duration;

use eyre::eyre;
use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

use super::commands::VerifyArgs;
use super::metrics::{FailureCategory, VerificationReport};
use crate::config::Config;

/// Verify transactions and return verification report.
#[allow(clippy::too_many_lines)]
pub(crate) async fn verify_transactions_with_report(
    args: VerifyArgs,
    config: &Config,
    skip_gmp: bool,
) -> eyre::Result<VerificationReport> {
    if args.resume_from == 0 {
        return Err(eyre!("--resume-from is 1-based and must be at least 1"));
    }

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

    let append_mode = args.resume_from > 1;

    for path in [
        &args.fail_output,
        &args.pending_output,
        &args.success_output,
    ] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut fail_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(!append_mode)
        .append(append_mode)
        .open(&args.fail_output)?;

    let mut pending_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(!append_mode)
        .append(append_mode)
        .open(&args.pending_output)?;

    let mut success_file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(!append_mode)
        .append(append_mode)
        .open(&args.success_output)?;

    let rpc_client = RpcClient::new_with_commitment(&config.url, CommitmentConfig::confirmed());

    let mut successful = 0u64;
    let mut failed = 0u64;
    let mut pending = 0u64;
    let mut failure_reasons: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();

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
                *failure_reasons
                    .entry("invalid signature format".to_owned())
                    .or_insert(0) += 1;
                failed += 1;
                continue;
            }
        };

        match verify_single_transaction(&signature, &rpc_client, config, skip_gmp).await {
            VerificationResult::Success => {
                writeln!(success_file, "{tx_str}")?;
                println!("\u{2713} Transaction verified successfully");
                successful += 1;
            }
            VerificationResult::Pending(msg) => {
                writeln!(pending_file, "{tx_str} : {msg}")?;
                println!("\u{23f3} Transaction pending: {msg}");
                pending += 1;
            }
            VerificationResult::Failed(msg) => {
                writeln!(fail_file, "{tx_str} : {msg}")?;
                println!("\u{2717} Transaction failed: {msg}");
                *failure_reasons.entry(categorize_error(&msg)).or_insert(0) += 1;
                failed += 1;
            }
        }

        tokio::time::sleep(Duration::from_millis(args.delay)).await;
    }

    let total_verified = successful + failed + pending;
    #[allow(clippy::cast_precision_loss, clippy::float_arithmetic)]
    let success_rate = if total_verified > 0 {
        successful as f64 / total_verified as f64
    } else {
        0.0
    };

    let failure_categories: Vec<FailureCategory> = failure_reasons
        .into_iter()
        .map(|(reason, count)| FailureCategory { reason, count })
        .collect();

    println!("\n========================================");
    println!("Verification completed!");
    println!("Successful: {successful}");
    println!("Failed: {failed}");
    println!("Pending: {pending}");
    #[allow(clippy::float_arithmetic)]
    let success_pct = success_rate * 100.0;
    println!("Success rate: {success_pct:.1}%");
    println!("========================================\n");

    Ok(VerificationReport {
        total_verified,
        successful,
        pending,
        failed,
        success_rate,
        failure_reasons: failure_categories,
    })
}

fn categorize_error(error: &str) -> String {
    if error.contains("Solana RPC") {
        "Solana RPC error".to_owned()
    } else if error.contains("Solana transaction error") {
        "Solana transaction error".to_owned()
    } else if error.contains("not found") {
        "Transaction not found".to_owned()
    } else if error.contains("Axelarscan") {
        "Axelarscan error".to_owned()
    } else if error.contains("GMP status") {
        "GMP not executed".to_owned()
    } else {
        "Other".to_owned()
    }
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
    skip_gmp: bool,
) -> VerificationResult {
    use solana_transaction_status::UiTransactionEncoding;

    match rpc_client.get_transaction(signature, UiTransactionEncoding::Json) {
        Ok(tx) => {
            if let Some(meta) = tx.transaction.meta {
                if let Some(err) = meta.err {
                    return VerificationResult::Failed(format!(
                        "Solana transaction error: {err:?}"
                    ));
                }
            }
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("not found") || err_str.contains("Transaction version") {
                return VerificationResult::Pending(
                    "Solana transaction not found or not finalized".to_owned(),
                );
            }
            return VerificationResult::Failed(format!("Solana RPC error: {e}"));
        }
    }

    if skip_gmp {
        return VerificationResult::Success;
    }

    match verify_axelarscan_gmp(signature, config).await {
        Ok(GmpStatus::Executed) => VerificationResult::Success,
        Ok(GmpStatus::Pending(status)) => {
            VerificationResult::Pending(format!("GMP status: {status}"))
        }
        Ok(GmpStatus::Failed(status)) => {
            VerificationResult::Failed(format!("GMP failed with status: {status}"))
        }
        Err(e) => VerificationResult::Failed(format!("Axelarscan verification error: {e}")),
    }
}

/// GMP verification status from Axelarscan.
enum GmpStatus {
    /// Transaction has been fully executed on destination chain.
    Executed,
    /// Transaction is still in progress (pending, confirming, approved, etc.).
    Pending(String),
    /// Transaction has definitively failed.
    Failed(String),
}

/// Known intermediate GMP statuses that indicate the transaction is still in progress.
const PENDING_STATUSES: &[&str] = &[
    "pending",
    "confirming",
    "approved",
    "approving",
    "executing",
    "gas_paid",
    "gas_paid_not_enough_gas",
    "called",
];

/// Known terminal failure statuses.
const FAILED_STATUSES: &[&str] = &["error", "failed", "reverted", "insufficient_fee"];

async fn verify_axelarscan_gmp(signature: &Signature, config: &Config) -> eyre::Result<GmpStatus> {
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

    let status_lower = status.to_lowercase();

    if status_lower == "executed" {
        return Ok(GmpStatus::Executed);
    }

    if PENDING_STATUSES.iter().any(|s| status_lower == *s) {
        return Ok(GmpStatus::Pending(status.to_owned()));
    }

    if FAILED_STATUSES.iter().any(|s| status_lower == *s) {
        return Ok(GmpStatus::Failed(status.to_owned()));
    }

    // Unknown status - treat as pending to avoid false failures
    Ok(GmpStatus::Pending(status.to_owned()))
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
