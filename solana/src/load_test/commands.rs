//! Command definitions and argument structures for load testing.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use eyre::eyre;

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Run outbound load test (ITS interchain transfers)
    Test(TestArgs),
    /// Verify transactions from a previous load test
    Verify(VerifyArgs),
    /// Run complete load test: test + verify + report
    Run(RunArgs),
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub(crate) enum ContentionMode {
    /// Round-robin across derived keypairs (default)
    #[default]
    None,
    /// All transactions from a single keypair (nonce contention stress)
    SingleAccount,
    /// Fire all transactions in parallel without delay
    Parallel,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct TestArgs {
    /// Signing keypair path. Defaults to Solana CLI config keypair.
    #[clap(long, env = "SOLANA_PRIVATE_KEY")]
    pub fee_payer: Option<String>,

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

    /// Test duration in seconds
    #[clap(long)]
    pub time: u64,

    /// Delay between transactions in milliseconds
    #[clap(long, default_value = "10")]
    pub delay: u64,

    #[clap(long, env = "MNEMONIC")]
    pub mnemonic: Option<String>,

    #[clap(long, env = "DERIVE_ACCOUNTS")]
    pub addresses_to_derive: Option<usize>,

    /// Contention testing mode
    #[clap(long, value_enum, default_value = "none")]
    pub contention_mode: ContentionMode,

    /// Optional payload/metadata to include (hex encoded)
    #[clap(long)]
    pub payload: Option<String>,

    /// Vary payload size randomly up to this many bytes
    #[clap(long)]
    pub vary_payload: Option<usize>,

    #[clap(long, default_value = "output/load-test.txt")]
    pub output: PathBuf,

    /// Output metrics as JSON
    #[clap(long, default_value = "output/load-test-metrics.json")]
    pub metrics_output: PathBuf,
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
    #[clap(long, default_value = "1")]
    pub resume_from: usize,

    #[clap(long, default_value = "100")]
    pub delay: u64,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct RunArgs {
    /// Signing keypair path. Defaults to Solana CLI config keypair.
    #[clap(long, env = "SOLANA_PRIVATE_KEY")]
    pub fee_payer: Option<String>,

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

    /// Test duration in seconds
    #[clap(long)]
    pub time: u64,

    /// Delay between transactions in milliseconds
    #[clap(long, default_value = "10")]
    pub delay: u64,

    #[clap(long, env = "MNEMONIC")]
    pub mnemonic: Option<String>,

    #[clap(long, env = "DERIVE_ACCOUNTS")]
    pub addresses_to_derive: Option<usize>,

    /// Contention testing mode
    #[clap(long, value_enum, default_value = "none")]
    pub contention_mode: ContentionMode,

    /// Optional payload/metadata to include (hex encoded)
    #[clap(long)]
    pub payload: Option<String>,

    /// Vary payload size randomly up to this many bytes
    #[clap(long)]
    pub vary_payload: Option<usize>,

    /// Output directory for all results
    #[clap(long, default_value = "output")]
    pub output_dir: PathBuf,

    /// Delay between verification requests in milliseconds
    #[clap(long, default_value = "100")]
    pub verify_delay: u64,

    /// Skip cross-chain verification (only check Solana confirmation)
    #[clap(long)]
    pub skip_gmp_verify: bool,
}

fn parse_hex_bytes32(s: &str) -> eyre::Result<[u8; 32]> {
    let decoded: [u8; 32] = hex::decode(s.trim_start_matches("0x"))?
        .try_into()
        .map_err(|_| eyre!("Invalid hex string length. Expected 32 bytes."))?;
    Ok(decoded)
}
