use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Configuration {
    // /// Account address to pay for Axelar TXs
    // #[arg(long)]
    // pub axelar_payer_addr: String,
    //
    /// Solana payer keypair file location
    #[arg(long)]
    pub solana_keypair_file: String,

    /// Solana chain name used around the protocol as source_chain, chain_name, etc
    #[arg(long)]
    pub solana_chain_name: String,

    /// The address of the Axelar Gateway smart contract, hosted on Solana
    #[arg(long)]
    pub axl_gw_addr_on_solana: String,

    /// Solana node WebSocket endpoint
    #[arg(long)]
    pub solana_ws: String,

    /// Solana node RPC endpoint
    #[arg(long)]
    pub solana_rpc: String,

    /// Axelar Amplifier API RPC URL
    #[arg(long)]
    pub amplifier_rpc: String,

    /// Postgres database URL
    #[arg(long)]
    pub database_url: String,
}
