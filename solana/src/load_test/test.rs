//! Load test execution logic.

use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use eyre::eyre;
use futures::future::join_all;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use solana_transaction_status::UiTransactionEncoding;
use tokio::sync::Mutex;

use super::commands::{ContentionMode, TestArgs};
use super::helpers::{
    derive_keypairs_from_mnemonic, get_associated_token_address, get_mint_from_token_manager,
    get_token_program_from_mint,
};
use super::metrics::{LoadTestReport, TxMetrics};
use crate::config::Config;
use crate::its;

/// Run load test (entry point for Test command).
pub(crate) async fn run_load_test(args: TestArgs, config: &Config) -> eyre::Result<()> {
    let _report = run_load_test_with_metrics(args, config).await?;
    Ok(())
}

/// Run load test and return metrics report.
#[allow(clippy::too_many_lines, clippy::float_arithmetic)]
pub(crate) async fn run_load_test_with_metrics(
    args: TestArgs,
    config: &Config,
) -> eyre::Result<LoadTestReport> {
    println!("Starting load test...");
    println!("Destination chain: {}", args.destination_chain);
    println!("Token ID: {}", hex::encode(args.token_id));
    println!("Transfer amount: {}", args.transfer_amount);
    println!("Duration: {} seconds", args.time);
    println!("Delay: {} ms", args.delay);
    println!("Contention mode: {:?}", args.contention_mode);
    println!("Output file: {}", args.output.display());

    for path in [&args.output, &args.metrics_output] {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
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

    let num_keypairs = keypairs.len();
    println!("Derived {num_keypairs} keypairs for testing");

    let start_time = Instant::now();
    let duration = Duration::from_secs(args.time);
    let delay_duration = Duration::from_millis(args.delay);

    let metrics_list: Arc<Mutex<Vec<TxMetrics>>> = Arc::new(Mutex::new(Vec::new()));
    let mut pending_tasks = Vec::new();

    let mut keypair_index = 0;
    let test_start = Instant::now();

    match args.contention_mode {
        ContentionMode::Parallel => loop {
            if start_time.elapsed() >= duration {
                break;
            }

            for keypair in &keypairs {
                let keypair = Arc::clone(keypair);
                let config_clone = config.clone();
                let args_clone = args.clone();
                let output_file_clone = Arc::clone(&output_file);
                let metrics_clone = Arc::clone(&metrics_list);

                let handle = tokio::spawn(async move {
                    execute_and_record(
                        keypair,
                        &args_clone,
                        config_clone,
                        output_file_clone,
                        metrics_clone,
                    )
                    .await;
                });
                pending_tasks.push(handle);
            }

            tokio::time::sleep(delay_duration).await;
        },
        ContentionMode::SingleAccount => {
            let single_keypair = Arc::clone(&keypairs[0]);
            loop {
                if start_time.elapsed() >= duration {
                    break;
                }

                let keypair = Arc::clone(&single_keypair);
                let config_clone = config.clone();
                let args_clone = args.clone();
                let output_file_clone = Arc::clone(&output_file);
                let metrics_clone = Arc::clone(&metrics_list);

                let handle = tokio::spawn(async move {
                    execute_and_record(
                        keypair,
                        &args_clone,
                        config_clone,
                        output_file_clone,
                        metrics_clone,
                    )
                    .await;
                });
                pending_tasks.push(handle);
                tokio::time::sleep(delay_duration).await;
            }
        }
        ContentionMode::None => loop {
            if start_time.elapsed() >= duration {
                break;
            }

            if keypair_index >= keypairs.len() {
                keypair_index = 0;
            }

            let keypair = Arc::clone(&keypairs[keypair_index]);
            keypair_index += 1;

            let config_clone = config.clone();
            let args_clone = args.clone();
            let output_file_clone = Arc::clone(&output_file);
            let metrics_clone = Arc::clone(&metrics_list);

            let handle = tokio::spawn(async move {
                execute_and_record(
                    keypair,
                    &args_clone,
                    config_clone,
                    output_file_clone,
                    metrics_clone,
                )
                .await;
            });
            pending_tasks.push(handle);
            tokio::time::sleep(delay_duration).await;
        },
    }

    let total_submitted = pending_tasks.len() as u64;
    let test_duration = test_start.elapsed().as_secs_f64();

    println!(
        "Waiting for {} pending transactions to complete...",
        pending_tasks.len()
    );
    join_all(pending_tasks).await;

    let metrics = metrics_list.lock().await.clone();
    let total_confirmed = metrics.iter().filter(|m| m.success).count() as u64;
    let total_failed = metrics.iter().filter(|m| !m.success).count() as u64;

    let latencies: Vec<u64> = metrics.iter().filter_map(|m| m.latency_ms).collect();
    let compute_units: Vec<u64> = metrics.iter().filter_map(|m| m.compute_units).collect();

    #[allow(clippy::cast_precision_loss)]
    let report = LoadTestReport {
        destination_chain: args.destination_chain,
        token_id: hex::encode(args.token_id),
        transfer_amount: args.transfer_amount,
        duration_secs: args.time,
        delay_ms: args.delay,
        contention_mode: format!("{:?}", args.contention_mode),
        num_keypairs,
        total_submitted,
        total_confirmed,
        total_failed,
        test_duration_secs: test_duration,
        tps_submitted: if test_duration > 0.0 {
            total_submitted as f64 / test_duration
        } else {
            0.0
        },
        tps_confirmed: if test_duration > 0.0 {
            total_confirmed as f64 / test_duration
        } else {
            0.0
        },
        landing_rate: if total_submitted > 0 {
            total_confirmed as f64 / total_submitted as f64
        } else {
            0.0
        },
        avg_latency_ms: if latencies.is_empty() {
            None
        } else {
            Some(latencies.iter().sum::<u64>() as f64 / latencies.len() as f64)
        },
        min_latency_ms: latencies.iter().min().copied(),
        max_latency_ms: latencies.iter().max().copied(),
        avg_compute_units: if compute_units.is_empty() {
            None
        } else {
            Some(compute_units.iter().sum::<u64>() as f64 / compute_units.len() as f64)
        },
        min_compute_units: compute_units.iter().min().copied(),
        max_compute_units: compute_units.iter().max().copied(),
        verification: None,
        transactions: metrics,
    };

    let metrics_json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&args.metrics_output, metrics_json)?;

    println!("\n========================================");
    println!("Load test completed!");
    println!("Total submitted: {}", report.total_submitted);
    println!("Total confirmed: {}", report.total_confirmed);
    println!("Total failed: {}", report.total_failed);
    println!("Test duration: {:.2} seconds", report.test_duration_secs);
    println!("TPS (submitted): {:.2}", report.tps_submitted);
    println!("TPS (confirmed): {:.2}", report.tps_confirmed);
    println!("Landing rate: {:.1}%", report.landing_rate * 100.0);
    if let Some(avg) = report.avg_latency_ms {
        println!("Avg latency: {avg:.1} ms");
    }
    if let Some(avg) = report.avg_compute_units {
        println!("Avg compute units: {avg:.0}");
    }
    println!("Metrics saved to: {}", args.metrics_output.display());
    println!("========================================\n");

    Ok(report)
}

#[allow(clippy::semicolon_outside_block)]
async fn execute_and_record(
    keypair: Arc<dyn Signer + Send + Sync>,
    args: &TestArgs,
    config: Config,
    output_file: Arc<Mutex<File>>,
    metrics_list: Arc<Mutex<Vec<TxMetrics>>>,
) {
    let submit_start = Instant::now();
    let result = execute_transfer_with_metrics(keypair, args, config).await;

    match result {
        Ok(metrics) => {
            {
                let mut file = output_file.lock().await;
                if let Err(e) = writeln!(file, "{}", metrics.signature) {
                    eprintln!("Failed to write signature to file: {e}");
                }
            }
            #[allow(clippy::string_slice)]
            let sig_prefix = &metrics.signature[..16];
            println!(
                "\u{2713} {} ({}ms, {} CU)",
                sig_prefix,
                metrics.latency_ms.unwrap_or(0),
                metrics.compute_units.unwrap_or(0)
            );
            metrics_list.lock().await.push(metrics);
        }
        Err(e) => {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed_ms = submit_start.elapsed().as_millis() as u64;
            let metrics = TxMetrics {
                signature: String::new(),
                submit_time_ms: elapsed_ms,
                confirm_time_ms: None,
                latency_ms: None,
                compute_units: None,
                slot: None,
                success: false,
                error: Some(e.to_string()),
            };
            eprintln!("\u{2717} Transaction failed: {e}");
            metrics_list.lock().await.push(metrics);
        }
    }
}

#[allow(clippy::unused_async)]
async fn execute_transfer_with_metrics(
    keypair: Arc<dyn Signer + Send + Sync>,
    args: &TestArgs,
    config: Config,
) -> eyre::Result<TxMetrics> {
    let submit_start = Instant::now();

    let source_account = {
        let mint = get_mint_from_token_manager(&args.token_id, &config)?;
        let token_program = get_token_program_from_mint(&mint, &config)?;
        get_associated_token_address(&keypair.pubkey(), &mint, &token_program)
    };

    let interchain_transfer_args = its::InterchainTransferArgs {
        source_account,
        token_id: args.token_id,
        destination_chain: args.destination_chain.clone(),
        destination_address: args.destination_address.clone(),
        amount: args.transfer_amount.clone(),
        gas_value: args.gas_value.unwrap_or(100_000),
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

    let instruction = instructions
        .into_iter()
        .next()
        .ok_or_else(|| eyre!("No instructions generated"))?;

    let message = solana_sdk::message::Message::new_with_blockhash(
        &[instruction],
        Some(&keypair.pubkey()),
        &blockhash,
    );
    let mut transaction = solana_sdk::transaction::Transaction::new_unsigned(message);

    let signers: Vec<&dyn Signer> = vec![keypair.as_ref()];
    transaction.sign(&signers, blockhash);

    #[allow(clippy::cast_possible_truncation)]
    let submit_time_ms = submit_start.elapsed().as_millis() as u64;

    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
    #[allow(clippy::cast_possible_truncation)]
    let confirm_time_ms = submit_start.elapsed().as_millis() as u64;
    let latency_ms = confirm_time_ms.saturating_sub(submit_time_ms);

    let (compute_units, slot) = fetch_tx_details(&rpc_client, &signature).unwrap_or((None, None));

    Ok(TxMetrics {
        signature: signature.to_string(),
        submit_time_ms,
        confirm_time_ms: Some(confirm_time_ms),
        latency_ms: Some(latency_ms),
        compute_units,
        slot,
        success: true,
        error: None,
    })
}

fn fetch_tx_details(
    rpc_client: &RpcClient,
    signature: &Signature,
) -> eyre::Result<(Option<u64>, Option<u64>)> {
    let tx = rpc_client.get_transaction(signature, UiTransactionEncoding::Json)?;

    let slot = Some(tx.slot);
    let compute_units = tx
        .transaction
        .meta
        .and_then(|m| Option::from(m.compute_units_consumed));

    Ok((compute_units, slot))
}
