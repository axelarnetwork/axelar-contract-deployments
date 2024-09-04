//! Utility crate for managing tasks, common commands,
//! deployments for the solana-axelar integration.

use std::sync::OnceLock;

use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::Layered;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, reload, EnvFilter, Registry};

mod cli;

static LOG_FILTER_HANDLE: OnceLock<
    reload::Handle<EnvFilter, Layered<fmt::Layer<Registry>, Registry>>,
> = OnceLock::new();

/// Change current subscriber log level
///
/// # Arguments
/// * `level` - `[LevelFilter]` with the new log level
///
/// # Panics
/// If reloading the log level filter fails
pub fn change_log_level(level: LevelFilter) {
    if let Some(handle) = crate::LOG_FILTER_HANDLE.get() {
        let env_filter = EnvFilter::builder()
            .with_default_directive(level.into())
            .from_env_lossy();
        handle
            .reload(env_filter)
            .expect("Error reloading log filter");
    }
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let cli = cli::Cli::try_parse()?;

    color_eyre::install()?;
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();

    let (filter_layer, handle) = reload::Layer::new(env_filter);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter_layer)
        .init();

    LOG_FILTER_HANDLE
        .set(handle)
        .expect("Error storing reload handle");

    cli.run().await
}
