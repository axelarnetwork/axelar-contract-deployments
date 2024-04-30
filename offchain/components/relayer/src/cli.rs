use std::path::PathBuf;

use clap::Parser;

/// Axelar-Solana Relayer.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the configuration file
    #[clap(short, long)]
    pub config: PathBuf,
}
