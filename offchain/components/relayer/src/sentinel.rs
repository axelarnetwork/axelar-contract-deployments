use crate::verifier::Verifier;
use crate::{entities::last_processed_block::Chain, state::PostgresStateTracker};
use amplifier_api::axl_rpc;
use futures_util::StreamExt;
use gmp_gateway::events::GatewayEvent;

use solana_client::{
    client_error,
    nonblocking::{pubsub_client, rpc_client::RpcClient},
    rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter},
};
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::commitment_config::{CommitmentConfig, CommitmentLevel};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

#[derive(Debug, Error)]
pub enum SentinelError {
    #[error("Solana RPC error - {0}")]
    SolanaRPCError(client_error::ClientError),
    #[error("Failed to subscribe for Solana logs - {0}")]
    SubForLogs(pubsub_client::PubsubClientError),
    #[error("Failed to parse vec<u8> into a String - {0}")]
    ByteVecParsing(std::string::FromUtf8Error),
    #[error("Failed to send an axelar Message to the gmp_sender mpsc channel - {0}")]
    GmpSenderBroadcast(mpsc::error::SendError<axl_rpc::Message>),
    #[error("Database error - {0}")]
    Database(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// listens for events coming
/// from the Axelar gateway smart contract on the Solana blockchain
///
/// Those will be messages sent from Solana dapps,
/// which pass through axelar and are sent to other blockchains.
pub struct Sentinel {
    source_chain: String,
    source_address: String,
    solana_rpc_client: Arc<RpcClient>,
    solana_pubsub_client: PubsubClient,
    verifier: Verifier,
    state: PostgresStateTracker,
}

impl Sentinel {
    /// Create a new sentinel, which listens for events coming
    /// from the Axelar gateway smart contract on the Solana blockchain
    fn new(
        source_chain: String,
        source_address: String,
        solana_rpc_client: Arc<RpcClient>,
        solana_pubsub_client: PubsubClient,
        verifier: Verifier,
        state: PostgresStateTracker,
    ) -> Self {
        Self {
            source_chain,
            source_address,
            solana_rpc_client,
            solana_pubsub_client,
            verifier,
            state,
        }
    }

    pub fn start(
        source_chain: String,
        source_address: String,
        solana_rpc_client: Arc<RpcClient>,
        solana_pubsub_client: PubsubClient,
        verifier: Verifier,
        state: PostgresStateTracker,
    ) {
        let sentinel = Self::new(
            source_chain,
            source_address,
            solana_rpc_client,
            solana_pubsub_client,
            verifier,
            state,
        );
        tokio::spawn(async move { sentinel.run().await });
    }

    /// Listens for gmp messages coming from the Axelar gateway smart contract on the Solana blockchain
    /// and forwards the messages through the gmp channel to the verifier
    async fn run(self) -> Result<(), SentinelError> {
        let start_height = self.state.load().await?;

        let current_slot = self
            .solana_rpc_client
            .get_slot()
            .await
            .map_err(SentinelError::SolanaRPCError)?;

        // TODO
        //
        // We might have to catch up with the current slot here
        //

        let (mut log_events, log_unsubscribe) = self
            .solana_pubsub_client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![self.source_address.clone()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig {
                        commitment: CommitmentLevel::Finalized,
                    }),
                },
            )
            .await
            .map_err(SentinelError::SubForLogs)?;

        while let Some(log) = log_events.next().await {
            // parse solana log to a GatewayEvent
            let gw_event_parsed: Option<GatewayEvent> =
                log.value.logs.into_iter().find_map(GatewayEvent::parse_log);

            // TODO: This is to be triggered every time a tx is sent to the gateway.
            // So maybe we should not log anything to not spam our logs
            // or should check if we can subscribe only for the txs which we care about
            let Some(gw_event_parsed) = gw_event_parsed else {
                // TODO: log error/warning that the logs were not parsed.
                // Do we care about program logs that failed to parse?
                warn!("not a GatewayEvent; skipping it");
                continue;
            };

            match gw_event_parsed {
                // GMP message to be sent to Axelar for verification
                GatewayEvent::CallContract {
                    sender: _,
                    destination_chain,
                    destination_address,
                    payload,
                    payload_hash: _,
                } => {
                    // TODO: Handle scenario with multiple messages (Issue #103)
                    let msg_id = format!("{}:{}:0", self.source_chain.clone(), log.value.signature);
                    info!("SENDING GMP MSG FOR VERIFICATION {}", msg_id);
                    // Construct an Axelar message and send it to Verifier for verification
                    self.verifier
                        .verify(axl_rpc::Message {
                            id: msg_id,
                            source_chain: self.source_chain.clone(),
                            source_address: self.source_address.clone(), //TODO: gw address, not sender, right?
                            destination_chain: String::from_utf8(destination_chain)
                                .map_err(SentinelError::ByteVecParsing)?,
                            // TODO: Should we hex encode it and prefix it with 0x?
                            destination_address: String::from_utf8(destination_address)
                                .map_err(SentinelError::ByteVecParsing)?,
                            payload,
                        })
                        .map_err(SentinelError::GmpSenderBroadcast)?;
                }
                // TODO: Handle event
                GatewayEvent::OperatorshipTransferred {
                    info_account_address: _,
                } => todo!(),
                _ => todo!(),
            }
        }

        Ok(())
    }
}
