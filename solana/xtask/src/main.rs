//! Utility crate for managing tasks, commond commands,
//! deployments for the solana-axelar integration.

use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

mod cli;

#[cfg(test)]
mod solana_tests;

#[cfg(test)]
mod test_helpers;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = cli::Cli::try_parse()?;

    color_eyre::install()?;

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    cli.run().await
}
