//! Load testing module for Solana ITS interchain transfers.

mod commands;
mod helpers;
mod metrics;
mod test;
mod verify;

pub(crate) use commands::Commands;

use commands::{RunArgs, TestArgs, VerifyArgs};
use metrics::LoadTestReport;

use crate::config::Config;

/// Handle load test commands.
pub(crate) async fn handle_command(command: Commands, config: &Config) -> eyre::Result<()> {
    match command {
        Commands::Test(args) => test::run_load_test(args, config).await,
        Commands::Verify(args) => verify::verify_transactions(args, config).await,
        Commands::Run(args) => run_full_test(args, config).await,
    }
}

/// Run complete load test: test + verify + report.
async fn run_full_test(args: RunArgs, config: &Config) -> eyre::Result<()> {
    std::fs::create_dir_all(&args.output_dir)?;

    let tx_output = args.output_dir.join("transactions.txt");
    let metrics_output = args.output_dir.join("metrics.json");
    let report_output = args.output_dir.join("report.json");
    let fail_output = args.output_dir.join("failed.txt");
    let pending_output = args.output_dir.join("pending.txt");
    let success_output = args.output_dir.join("success.txt");

    println!("\n{}", "=".repeat(60));
    println!("PHASE 1: LOAD TEST");
    println!("{}\n", "=".repeat(60));

    let test_args = TestArgs {
        destination_chain: args.destination_chain.clone(),
        token_id: args.token_id,
        destination_address: args.destination_address.clone(),
        transfer_amount: args.transfer_amount.clone(),
        gas_value: args.gas_value,
        time: args.time,
        delay: args.delay,
        mnemonic: args.mnemonic.clone(),
        addresses_to_derive: args.addresses_to_derive,
        contention_mode: args.contention_mode,
        payload: args.payload.clone(),
        vary_payload: args.vary_payload,
        output: tx_output.clone(),
        metrics_output: metrics_output.clone(),
    };

    let mut report = test::run_load_test_with_metrics(test_args, config).await?;

    println!("\n{}", "=".repeat(60));
    println!("PHASE 2: VERIFICATION");
    println!("{}\n", "=".repeat(60));

    let verify_args = VerifyArgs {
        input_file: tx_output,
        fail_output,
        pending_output,
        success_output,
        resume_from: 1,
        delay: args.verify_delay,
    };

    let verification =
        verify::verify_transactions_with_report(verify_args, config, args.skip_gmp_verify).await?;

    report.verification = Some(verification);

    println!("\n{}", "=".repeat(60));
    println!("PHASE 3: FINAL REPORT");
    println!("{}\n", "=".repeat(60));

    let report_json = serde_json::to_string_pretty(&report)?;
    std::fs::write(&report_output, &report_json)?;

    print_final_report(&report);
    println!("\nFull report saved to: {}", report_output.display());

    Ok(())
}

#[allow(clippy::non_ascii_literal, clippy::float_arithmetic)]
fn print_final_report(report: &LoadTestReport) {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║              COMPREHENSIVE LOAD TEST REPORT              ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ CONFIGURATION                                            ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ Destination Chain: {:>38} ║", report.destination_chain);
    println!("║ Transfer Amount:   {:>38} ║", report.transfer_amount);
    println!("║ Duration:          {:>35} s ║", report.duration_secs);
    println!("║ Delay:             {:>34} ms ║", report.delay_ms);
    println!("║ Contention Mode:   {:>38} ║", report.contention_mode);
    println!("║ Keypairs:          {:>38} ║", report.num_keypairs);
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ THROUGHPUT METRICS                                       ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ Total Submitted:   {:>38} ║", report.total_submitted);
    println!("║ Total Confirmed:   {:>38} ║", report.total_confirmed);
    println!("║ Total Failed:      {:>38} ║", report.total_failed);
    println!("║ TPS (Submitted):   {:>36.2} ║", report.tps_submitted);
    println!("║ TPS (Confirmed):   {:>36.2} ║", report.tps_confirmed);
    println!(
        "║ Landing Rate:      {:>35.1}% ║",
        report.landing_rate * 100.0
    );
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║ SOLANA METRICS                                           ║");
    println!("╠══════════════════════════════════════════════════════════╣");

    if let Some(avg) = report.avg_latency_ms {
        println!("║ Avg Latency:       {avg:>34.1} ms ║");
    }
    if let (Some(min), Some(max)) = (report.min_latency_ms, report.max_latency_ms) {
        println!("║ Latency Range:     {min:>23} - {max} ms ║");
    }
    if let Some(avg) = report.avg_compute_units {
        println!("║ Avg Compute Units: {avg:>36.0} ║");
    }
    if let (Some(min), Some(max)) = (report.min_compute_units, report.max_compute_units) {
        println!("║ CU Range:          {min:>25} - {max} ║");
    }

    if let Some(ref v) = report.verification {
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ CROSS-CHAIN VERIFICATION                                 ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Verified:          {:>38} ║", v.total_verified);
        println!("║ Successful:        {:>38} ║", v.successful);
        println!("║ Pending:           {:>38} ║", v.pending);
        println!("║ Failed:            {:>38} ║", v.failed);
        println!("║ Success Rate:      {:>35.1}% ║", v.success_rate * 100.0);

        if !v.failure_reasons.is_empty() {
            println!("╠──────────────────────────────────────────────────────────╣");
            println!("║ Failure Breakdown:                                       ║");
            for cat in &v.failure_reasons {
                println!("║   {:.<45} {:>6} ║", cat.reason, cat.count);
            }
        }
    }

    println!("╚══════════════════════════════════════════════════════════╝");
}
