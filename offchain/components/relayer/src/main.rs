use amplifier_api::axl_rpc::axelar_rpc_client::AxelarRpcClient;
use approver::Approver;
use clap::Parser;
use config::RelayerConfig;
use sentinel::Sentinel;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::signer::keypair::read_keypair_file;
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::{error, info};
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    fmt::{self},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
use verifier::Verifier;

mod approver;
mod config;
mod sentinel;
mod verifier;

pub fn init_logging() {
    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env_lossy();

    let stdout = fmt::layer()
        .compact()
        // .with_file(true)
        // .with_line_number(true)
        // .with_thread_ids(true)
        // .with_target(true)
        // .with_span_events(FmtSpan::CLOSE)
        .with_writer(std::io::stdout);

    tracing_subscriber::registry()
        .with(filter)
        .with(stdout)
        .init();
}

#[tokio::main]
async fn main() {
    init_logging();

    info!("Starting server");

    start_workers().await;
}

async fn start_workers() {
    let config = RelayerConfig::parse();
    let (amplifier_rpc_client, solana_rpc_client, solana_pubsub_client) =
        start_clients(&config).await;
    // Workers
    // The broadcaster waits for ready for broadcasting solana txs, on the broadcast channel
    let payer_keypair = Arc::new(
        read_keypair_file(&config.solana_keypair_file)
            .expect("Failed to read keypair file: {error}"),
    );
    let verifier = Verifier::start(amplifier_rpc_client.clone());
    Sentinel::start(
        config.solana_chain_name.clone(),
        config.axl_gw_addr_on_solana.clone(),
        solana_pubsub_client,
        verifier,
    );
    let mut approver = Approver::new(
        config.solana_chain_name.clone(),
        amplifier_rpc_client.clone(),
        solana_rpc_client.clone(),
        payer_keypair.clone(),
    );

    match approver.run().await {
        Ok(_) => (),
        Err(e) => error!("failed to run approver {e}"),
    }
}

async fn start_clients(
    config: &RelayerConfig,
) -> (AxelarRpcClient<Channel>, Arc<RpcClient>, PubsubClient) {
    let amplifier_rpc_client = AxelarRpcClient::connect(config.amplifier_rpc.clone())
        .await
        .expect("Failed to create Amplifier RPC client");
    let solana_rpc_client = Arc::new(RpcClient::new(config.solana_rpc.clone()));
    let solana_pubsub_client = PubsubClient::new(&config.solana_ws)
        .await
        .expect("Failed to create Solana PubSub client");

    (
        amplifier_rpc_client,
        solana_rpc_client,
        solana_pubsub_client,
    )
}
