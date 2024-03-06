use crate::config::SOLANA_CHAIN_NAME;
use crate::sentinel::error::SentinelError;
use crate::sentinel::transaction_scanner::TransactionScanner;
use crate::sentinel::types::{
    SolanaTransaction,
    TransactionScannerMessage::{Message, Terminated},
};
use crate::{state::State, tokio_utils::handle_join_error};
use amplifier_api::axl_rpc;
use gmp_gateway::events::GatewayEvent;
use gmp_gateway::types::PubkeyWrapper;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::time::Duration;
use tokio::sync::mpsc::Sender;
use tracing::{error, info, trace, warn};
use url::Url;

mod error;
mod transaction_scanner;
mod types;

// TODO: All those contants should be configurable
const FETCH_SIGNATURES_INTERVAL: Duration = Duration::from_secs(5);

/// Solana Sentinel
///
/// Monitors the Solana Gateway program for relevant events.
pub struct SolanaSentinel {
    gateway_address: Pubkey,
    rpc: Url,
    verifier_channel: Sender<axl_rpc::Message>,
    state: State,
}

impl SolanaSentinel {
    pub fn new(
        gateway_address: Pubkey,
        rpc: Url,
        verifier_channel: Sender<axl_rpc::Message>,
        state: State,
    ) -> Self {
        Self {
            gateway_address,
            rpc,
            verifier_channel,
            state,
        }
    }

    #[tracing::instrument(name = "solana-sentinel", skip(self))]
    pub async fn run(self) {
        info!("task started");
        match tokio::spawn(self.work()).await {
            Ok(Ok(())) => {
                warn!("worker returned without errors");
            }
            Ok(Err(sentinel_error)) => {
                error!(%sentinel_error);
            }
            Err(join_error) => handle_join_error(join_error),
        };
    }

    /// Listens to Gateway program logs and forward them to the Axelar Verifier worker.
    async fn work(self) -> Result<(), SentinelError> {
        let (transaction_scanner_handle, mut transaction_receiver) = TransactionScanner::start(
            self.gateway_address,
            self.state.clone(),
            self.rpc.clone(),
            FETCH_SIGNATURES_INTERVAL,
        );

        // Listens for incoming Solana transactions and process them sequentially to propperly update the
        // latest known transaction signature.
        // TODO: use recv_many() to increase throughput and register the latest known signature only once per call.
        while let Some(message) = transaction_receiver.recv().await {
            let rpc_result = match message {
                Message(join_handle) => join_handle.await?,
                Terminated(error) => {
                    error!(error = %error, "Transaction scanner terminated");
                    transaction_scanner_handle.abort();
                    return Err(*error);
                }
            };

            // Resolve the outcome of the fetch transaction RPC call
            match rpc_result.map_err(|boxed| *boxed) {
                Err(SentinelError::NonFatal(non_fatal_error)) => {
                    // Don't halt operation for non-fatal errors
                    warn!(error = %non_fatal_error, r#type = "non-fatal")
                }
                Err(other) => return Err(other),
                Ok(solana_transaction) => self.process_transaction(solana_transaction).await?,
            }
        }

        // This function should never reach this point. If it ever does, return an error.
        Err(SentinelError::Stopped)
    }

    async fn process_transaction(
        &self,
        solana_transaction: SolanaTransaction,
    ) -> Result<(), SentinelError> {
        trace!(transaction_sig = %solana_transaction.signature);
        let gateway_events = solana_transaction
            .logs
            .iter()
            .enumerate() // Enumerate before filtering to keep indices consistent
            .filter_map(|(tx_index, log)| {
                GatewayEvent::parse_log(log).map(|event| (tx_index, event))
            });

        for (tx_index, event) in gateway_events {
            match event {
                GatewayEvent::CallContract {
                    destination_chain,
                    destination_address,
                    payload,
                    sender,
                    ..
                } => {
                    self.handle_gateway_call_contract_event(
                        solana_transaction.signature,
                        tx_index,
                        sender,
                        destination_chain,
                        destination_address,
                        payload,
                    )
                    .await?
                }

                GatewayEvent::OperatorshipTransferred {
                    info_account_address: _,
                } => todo!("Handle Operatorship Transferred event"),
                _ => unimplemented!(),
            };
        }

        // Mark this as the latest seen solana transaction
        self.state
            .update_solana_transaction(solana_transaction.signature)
            .await
            .map_err(Into::into)
    }

    async fn handle_gateway_call_contract_event(
        &self,
        transaction_signature: Signature,
        transaction_index: usize,
        sender: PubkeyWrapper,
        destination_chain: Vec<u8>,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
    ) -> Result<(), SentinelError> {
        let message_ccid = format!(
            "{}:{}:{}",
            SOLANA_CHAIN_NAME, transaction_signature, transaction_index,
        );
        let message = axl_rpc::Message {
            id: message_ccid,
            source_chain: SOLANA_CHAIN_NAME.into(),
            source_address: hex::encode(sender.to_bytes()),
            destination_chain: String::from_utf8(destination_chain)
                .map_err(SentinelError::ByteVecParsing)?,
            destination_address: hex::encode(destination_address),
            payload,
        };
        info!(?message);

        self.verifier_channel
            .send(message)
            .await
            .map_err(|message| SentinelError::SendMessageError(message.0.id))
    }
}
