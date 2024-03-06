use crate::sentinel::types::{SolanaTransaction, TransactionScannerMessage};
use crate::sentinel::SentinelError;
use crate::state::State;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Semaphore;
use tokio::task::{self, AbortHandle, JoinHandle};
use tracing::{error, trace_span, warn, Instrument};
use url::Url;

// TODO: All those contants should be configurable
const SIGNATURE_CHANNEL_CAPACITY: usize = 1_000;
const TRANSACTION_CHANNEL_CAPACITY: usize = 1_000;
const MAX_CONCURRENT_RPC_REQUESTS: usize = 20;

pub struct TransactionScanner {
    gateway_address: Pubkey,
    rpc: Url,
    state: State,
    sender: Sender<TransactionScannerMessage>,
    fetch_signatures_interval: Duration,
}

impl TransactionScanner {
    pub fn start(
        gateway_address: Pubkey,
        state: State,
        rpc: Url,
        fetch_signatures_interval: Duration,
    ) -> (AbortHandle, Receiver<TransactionScannerMessage>) {
        let (sender, receiver) = mpsc::channel(TRANSACTION_CHANNEL_CAPACITY);
        let worker = TransactionScanner {
            gateway_address,
            rpc,
            state,
            sender,
            fetch_signatures_interval,
        };
        let abort = tokio::spawn(async move { worker.run().await }).abort_handle();
        (abort, receiver)
    }

    #[tracing::instrument(name = "transaction-scanner", skip(self))]
    async fn run(&self) {
        let (signature_sender, signature_receiver) =
            mpsc::channel::<Signature>(SIGNATURE_CHANNEL_CAPACITY);

        // Start the pipeline tasks/futures
        let signature_scanner = signature_scanner::run(
            self.gateway_address,
            self.rpc.clone(),
            self.state.clone(),
            signature_sender,
            self.fetch_signatures_interval,
        );

        let transaction_retriever =
            transaction_retriever::run(self.rpc.clone(), signature_receiver, self.sender.clone());

        // Both tasks should never terminate, so if the program ever reach this point
        // it means the operation should be aborted.

        let termination = |error: SentinelError| async {
            self.sender
                .send(TransactionScannerMessage::Terminated(Box::new(error)))
                .await
                .expect("failed to send termination message");
        };

        tokio::select! {
            res = signature_scanner => {
                let error = res.unwrap_or(SentinelError::TransactionScannerStopped);
                termination(error).await;
            },
            () = transaction_retriever => {
                termination(SentinelError::TransactionScannerStopped).await;
            }
        };
    }
}

/// Functions to obtain transaction signatures from Solana RPC.
mod signature_scanner {
    use super::*;

    /// Continously fetches signatures from RPC and pipe them over a channel to further processing.
    #[tracing::instrument(skip_all)]
    pub fn run(
        gateway_address: Pubkey,
        url: Url,
        state: State,
        signature_sender: Sender<Signature>,
        period: Duration,
    ) -> JoinHandle<SentinelError> {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        task::spawn(
            async move {
                loop {
                    if let Err(error) = collect_and_process_signatures(
                        gateway_address,
                        url.clone(),
                        state.clone(),
                        signature_sender.clone(),
                    )
                    .await
                    {
                        error!(%error);
                        break error;
                    };
                    interval.tick().await;
                }
            }
            .instrument(trace_span!("solana-signature-scanner")),
        )
    }

    /// Calls Solana RPC after relevant transaction signatures and send results over a channel.
    #[tracing::instrument(skip_all)]
    async fn collect_and_process_signatures(
        gateway_address: Pubkey,
        url: Url,
        state: State,
        signature_sender: Sender<Signature>,
    ) -> Result<(), SentinelError> {
        let rpc_client = RpcClient::new(url.to_string());

        // Collect Signatures until exhaustion
        let last_known_signature: Option<Signature> = state.get_solana_transaction().await?;
        let collected_signatures =
            fetch_signatures_until_exhaustion(rpc_client, gateway_address, last_known_signature)
                .await?;

        // Iterate backwards so oldest signatures are picked up first on the other end.
        for signature in collected_signatures.into_iter().rev() {
            signature_sender.send(signature).await?;
        }
        Ok(())
    }

    /// Calls Solana RPC potentially multiple times until we get all undiscovered transactions.
    #[tracing::instrument(skip(rpc_client, gateway_address))]
    async fn fetch_signatures_until_exhaustion(
        rpc_client: RpcClient,
        gateway_address: Pubkey,
        until: Option<Signature>,
    ) -> Result<Vec<Signature>, SentinelError> {
        const LIMIT: usize = 1_000;

        // Helper function to setup the configuration at each loop
        let config = |before: Option<Signature>| GetConfirmedSignaturesForAddress2Config {
            before,
            until,
            limit: Some(LIMIT),
            commitment: Some(CommitmentConfig::finalized()),
        };

        let mut collected_signatures = vec![];
        let mut last_visited: Option<Signature> = None;
        loop {
            let batch = rpc_client
                .get_signatures_for_address_with_config(&gateway_address, config(last_visited))
                .await?;

            // Get the last (oldest) signature on this batch or break if it is empty
            let Some(oldest) = batch.last() else { break };

            // Set up following calls to start from the point this one had left
            last_visited = Some(Signature::from_str(&oldest.signature)?);

            let batch_size = batch.len();
            collected_signatures.extend(batch.into_iter());

            // If the results are less than the limit, then it means we have all the signatures we need.
            if batch_size < LIMIT {
                break;
            };
        }

        Ok(collected_signatures
            .into_iter()
            .map(|status| Signature::from_str(&status.signature))
            .collect::<Result<Vec<_>, _>>()?)
    }
}

/// Functions to resolve transaction signatures into full transactions, with metadata.
mod transaction_retriever {
    use super::*;
    use crate::sentinel::error::NonFatalError;
    use futures_util::TryFutureExt;
    use solana_transaction_status::{
        option_serializer::OptionSerializer, EncodedConfirmedTransactionWithStatusMeta,
    };
    use tracing::{debug, info};
    use TransactionScannerMessage::Message;

    /// For each incoming transaction signature, spawns a Tokio task to fetch the full transaction
    /// information. Tasks wait for a semaphore permit before calling the Solana RPC endpoint.
    pub async fn run(
        url: Url,
        mut signature_receiver: Receiver<Signature>,
        transaction_sender: Sender<TransactionScannerMessage>,
    ) {
        let rpc_client = Arc::new(RpcClient::new(url.to_string()));
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_RPC_REQUESTS));

        while let Some(signature) = signature_receiver.recv().await {
            let future = fetch_with_permit(signature, rpc_client.clone(), semaphore.clone())
                .map_err(Box::new);
            let task = task::spawn(future);
            transaction_sender
                .send(Message(task))
                .await
                .expect("failed to send task");
        }
    }

    /// Calls `getTransactionWithConfig` RPC method and returns relevant fields of a Solana
    /// transaction.
    async fn fetch(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
    ) -> Result<SolanaTransaction, SentinelError> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        debug!(%signature, "fetching transaction");

        let EncodedConfirmedTransactionWithStatusMeta {
            block_time,
            slot,
            transaction: transaction_with_meta,
        } = rpc_client
            .get_transaction_with_config(&signature, config)
            .await?;

        info!(%signature, "fetched transaction");

        let decoded_transaction = transaction_with_meta
            .transaction
            .decode()
            .ok_or_else(|| SentinelError::TransactionDecode { signature })?;

        // Check: This is the transaction we asked
        if !decoded_transaction.signatures.contains(&signature) {
            Err(NonFatalError::WrongTransactionReceived {
                expected: signature,
                received: *decoded_transaction
                    .signatures
                    .first()
                    .expect("Solana transaction should have at least one signature"),
            })?;
        }

        let meta = transaction_with_meta
            .meta
            .ok_or(NonFatalError::TransactionWithoutMeta { signature })?;

        let OptionSerializer::Some(logs) = meta.log_messages else {
            Err(NonFatalError::TransactionWithoutLogs { signature })?
        };

        Ok(SolanaTransaction {
            signature,
            logs,
            block_time,
            slot,
        })
    }

    /// Calls the [fetch] function whenever a semaphore permit is acquired.
    async fn fetch_with_permit(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
        semaphore: Arc<Semaphore>,
    ) -> Result<SolanaTransaction, SentinelError> {
        // Unwrap: This can return an error if the semaphore is closed, but we
        // never close it, so this error can never happen.
        let _permit = semaphore.clone().acquire_owned().await.unwrap();
        fetch(signature, rpc_client).await
    }
}
