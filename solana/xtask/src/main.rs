use clap::Parser;
use cli::report::Report;

mod cli;

#[tokio::main]
async fn main() -> anyhow::Result<Report> {
    tracing_subscriber::fmt().init();
    let cli = cli::Cli::try_parse()?;
    cli.run().await
}
