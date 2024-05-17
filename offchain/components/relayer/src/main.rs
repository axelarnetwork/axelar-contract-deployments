use anyhow::anyhow;
use clap::Parser;
use cli::Cli;
use config::Config;
use tracing_subscriber::filter::{EnvFilter, LevelFilter};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::{self};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod amplifier_api;
mod approver;
mod cli;
mod config;
mod healthcheck;
mod includer;
mod relayer;
mod retrying_http_sender;
mod sentinel;
mod state;
mod transports;
mod verifier;

pub fn init_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    let stdout = fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(true)
        .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    let args = Cli::parse();
    let config = Config::from_file(&args.config)?;
    let relayer = crate::relayer::Relayer::from_config(config).await?;
    relayer.run().await?;
    Err(anyhow!("Terminated"))
}
