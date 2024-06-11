//! Utility crate for managing tasks, commond commands,
//! deployments for the solana-axelar integration.

use clap::Parser;

mod cli;

#[cfg(test)]
mod solana_tests;

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod evm_tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // todo: always have `INFO` level on by default
    tracing_subscriber::fmt().init();
    let cli = cli::Cli::try_parse()?;
    cli.run().await
}
