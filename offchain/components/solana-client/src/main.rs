use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::keypair::read_keypair_file;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use std::path::PathBuf;
use tiny_keccak::Hasher;
use tracing::{debug, error, info};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

pub fn init_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    let stdout = fmt::layer().with_target(false).with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, ValueEnum)]
enum Cluster {
    Localhost,
    Devnet,
}

impl Cluster {
    fn url(&self) -> &'static str {
        match self {
            Cluster::Localhost => "http://localhost:8899",
            Cluster::Devnet => "https://api.devnet.solana.com",
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Payload which will be sent to Solana Gateway e.g. "1, 2, 3, 4"
    #[arg(short, long)]
    payload: String,

    /// Symbol of destination chain e.g. "eth".
    #[arg(long)]
    destination_chain: String,

    /// Address of contract on destination chain e.g.
    /// "0x999991888887653456765445676544567654567765"
    #[arg(long)]
    destination_contract_address: String,

    /// Account address to pay for Axelar TXs
    #[arg(short, long)]
    solana_payer_path: PathBuf,

    #[arg(short, long)]
    cluster: Cluster,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_logging();
    let args = Args::parse();
    let payer = read_keypair_file(&args.solana_payer_path)
        .map_err(|error| anyhow!("Failed to read keypair file: {error}"))?;

    let client = RpcClient::new(args.cluster.url().to_string());

    debug!(cluster = ?args.cluster, gateway_program_id = %gmp_gateway::id(), "Program start");
    gateway_call_contract(
        &client,
        &payer,
        &args.destination_chain,
        args.destination_contract_address.as_bytes(),
        args.payload.as_ref(),
    )
    .await
}

async fn gateway_call_contract(
    client: &RpcClient,
    payer: &Keypair,
    destination_chain: &str,
    destination_contract_address: &[u8],
    payload: &[u8],
) -> anyhow::Result<()> {
    // Explination, why this is commented:
    // https://eig3r.slack.com/archives/C05TRKNJDQS/p1705935199156679?thread_ts=1705932493.150129&cid=C05TRKNJDQS
    // let payload_hash = solana_program::hash::hash(payload).to_bytes();
    let mut sha3 = tiny_keccak::Sha3::v256();
    let mut payload_hash = [0u8; 32];
    sha3.update(payload);
    sha3.finalize(&mut payload_hash);

    let ix = gmp_gateway::instructions::call_contract(
        gmp_gateway::id(),
        payer.pubkey(),
        destination_chain,
        destination_contract_address,
        payload,
        payload_hash,
    )?;

    let latest_blockhash = client.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &[payer],
        latest_blockhash,
    );

    debug!("Broadcasting the transaction");
    match client.send_and_confirm_transaction(&tx).await {
        Ok(signature) => info!(tx_hash = %signature, "Transaction successfully sent"),
        Err(error) => {
            error!(%error);
            std::process::exit(1)
        }
    }

    Ok(())
}
