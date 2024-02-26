use amplifier_api::axl_rpc::axelar_rpc_client::AxelarRpcClient;
use approver::Approver;
use clap::Parser;
use configuration::Configuration;
use migrations::MigratorTrait;
use sea_orm::{ColumnTrait, ColumnType, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use sentinel::Sentinel;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::signer::keypair::read_keypair_file;
use state::PostgresStateTracker;
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

use crate::entities::last_processed_block::{self, Chain};

mod approver;
mod configuration;
mod entities;
mod sentinel;
mod state;
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
    let config = Configuration::parse();

    init_logging();

    let database = setup_database(&config)
        .await
        .expect("Failed to setup database");

    info!("Starting server");

    start_workers(&config, database).await;
}

async fn setup_database(config: &Configuration) -> Result<sea_orm::DatabaseConnection, DbErr> {
    let database = sea_orm::Database::connect(config.database_url.clone())
        .await
        .expect("Failed to connect to database");

    migrations::Migrator::up(&database, None).await?;

    Ok(database)
}

async fn start_workers(config: &Configuration, database: DatabaseConnection) {
    let state = PostgresStateTracker::new(database);

    let (amplifier_rpc_client, solana_rpc_client, solana_pubsub_client) =
        start_clients(config).await;
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
        solana_rpc_client.clone(),
        solana_pubsub_client,
        verifier,
        state.clone(),
    );
    let mut approver = Approver::new(
        config.solana_chain_name.clone(),
        amplifier_rpc_client.clone(),
        solana_rpc_client.clone(),
        payer_keypair.clone(),
        state,
    );

    match approver.run().await {
        Ok(_) => (),
        Err(e) => error!("failed to run approver {e}"),
    }
}

async fn start_clients(
    config: &Configuration,
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
