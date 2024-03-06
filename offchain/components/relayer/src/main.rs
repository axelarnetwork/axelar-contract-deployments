mod approver;
mod config;
mod relayer;
mod sentinel;
mod state;
mod tokio_utils;
mod verifier;

use anyhow::anyhow;
use config::parse_command_line_args;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

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
    let config = parse_command_line_args()?;
    let relayer = crate::relayer::Relayer::from_config(config).await?;
    relayer.run().await;
    Err(anyhow!("Terminating"))
}
