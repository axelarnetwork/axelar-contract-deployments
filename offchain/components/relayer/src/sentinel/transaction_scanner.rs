use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::FutureExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program::pubkey::Pubkey;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use solana_transaction_status::UiTransactionEncoding;
use thiserror::Error;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tracing::{error, trace, warn};
use url::Url;

use crate::sentinel::error::TransactionScannerError;
use crate::sentinel::types::{SolanaTransaction, TransactionScannerMessage};
use crate::state::interface::State;

// TODO: All those contants should be configurable
const SIGNATURE_CHANNEL_CAPACITY: usize = 1_000;
const TRANSACTION_CHANNEL_CAPACITY: usize = 1_000;
const MAX_CONCURRENT_RPC_REQUESTS: usize = 20;

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("Transaction scanner received the cancellation signal")]
    Cancelled,
}

/// Scans a Solana program for relevant transactions and provide them in a tokio
/// channel.
///
/// The operation is orchestrated by two internal worker futures, defined in
/// these modules:
/// 1. [`signature_scanner`]
/// 2. [`transaction_retriever`]
///
/// [`signature_scanner`] runs in a loop `getConfirmedSignaturesForAddress`
/// Solana RPC endpoint in. As this method only returns the signatures, they are
/// sent through a private tokio channel to the [`transaction_retriever`]
/// future, which then spawns a tokio task to fetch the full transaction details
/// for every incoming signature.
///
/// Signatures are processed in chronological order, but calls to
/// `getTransaction` RPC endpoint happen concurrently. Final values are
/// transmitted back to the caller wrapped in the [`TransactionScannerMessage`]
/// type, which holds a [`tokio::spawn::JoinHandle`] with the results of the
/// async call to `getTransaction`.
pub struct TransactionScanner<S>
where
    S: State<Signature>,
{
    /// Solana program address to monitor relevant transactions.
    address: Pubkey,
    //// Solana RPC endpoint.
    rpc: Url,
    /// Database handle.
    state: S,
    /// Results channel used by the Solana Sentinel.
    sender: Sender<TransactionScannerMessage>,
    /// Wait time before scanning Solana RPC signature ranges in loop
    fetch_signatures_interval: Duration,
    /// Shutdown signal.
    cancellation_token: CancellationToken,
}

impl<S> TransactionScanner<S>
where
    S: State<Signature> + Clone,
{
    /// Returns a worker future and a receiver channel for polling transactions
    /// from a given Solana address.
    ///
    /// The returned future represents [`TransactionScanner::run`], which will
    /// fetch Solana transactions at a specified interval and send
    /// `TransactionScannerMessage` events through the returned `Receiver`.
    ///
    /// The returned future must be polled regularly, otherwise the receiving
    /// channel won't receive any messages.
    pub fn setup(
        address: Pubkey,
        state: S,
        rpc: Url,
        fetch_signatures_interval: Duration,
        cancellation_token: CancellationToken,
    ) -> (
        impl std::future::Future,
        Receiver<TransactionScannerMessage>,
    ) {
        let (sender, receiver) = mpsc::channel(TRANSACTION_CHANNEL_CAPACITY);
        let worker = TransactionScanner {
            address,
            rpc,
            state,
            sender,
            fetch_signatures_interval,
            cancellation_token,
        }
        .run()
        .fuse();
        (worker, receiver)
    }

    #[tracing::instrument(name = "transaction-scanner", skip(self))]
    async fn run(self) {
        let (signature_sender, signature_receiver) =
            mpsc::channel::<Signature>(SIGNATURE_CHANNEL_CAPACITY);
        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_RPC_REQUESTS));

        // Start the pipeline long-living futures.
        let signature_scanner = signature_scanner::run(
            self.address,
            self.rpc.clone(),
            self.state.clone(),
            signature_sender,
            self.fetch_signatures_interval,
            self.cancellation_token.clone(),
            semaphore.clone(),
        );

        let transaction_retriever = transaction_retriever::run(
            self.rpc.clone(),
            signature_receiver,
            self.sender.clone(),
            self.cancellation_token.clone(),
            semaphore,
        );

        // Signals cancellation and sends an error message back to the Solana Sentinel.
        let termination = |error: TransactionScannerError| async {
            self.cancellation_token.cancel();
            if let Err(send_error) = self
                .sender
                .send(TransactionScannerMessage::Terminated(error))
                .await
            {
                error!(%send_error, "Failed to send an error message back to the sentinel while terminating");
            };
        };

        tokio::select! {
            // Listen for the cancellation signal
            _ = self.cancellation_token.cancelled() => {
                trace!("cancelled");
                termination(InternalError::Cancelled.into()).await;
            }

            Err(error) = signature_scanner => {
                warn!(%error, source="signature-scanner", "terminating");
                termination(error.into()).await;
            },

            error = transaction_retriever => {
                warn!(source="signature-scanner", "terminating");
                termination(error.into()).await;
            }
        };
    }
}

/// Functions to obtain transaction signatures from Solana RPC.
pub mod signature_scanner {
    use std::convert::Infallible as Never;

    use solana_client::client_error::ClientError;
    use solana_sdk::signature::ParseSignatureError;
    use tokio::sync::mpsc::error::SendError;
    use tokio::sync::AcquireError;
    use tracing::trace;

    use super::*;

    #[derive(Error, Debug)]
    pub enum SignatureScannerError {
        #[error("Signature scanner received the cancellation signal")]
        Cancelled,
        #[error("Failed to decode base58 string as a Solana signature: {0}")]
        SignatureParse(#[from] ParseSignatureError),
        #[error(transparent)]
        SolanaClient(#[from] ClientError),
        #[error("Failed to send transaction signature for fetching its details: {0}")]
        SendSignatureError(#[from] SendError<Signature>),
        #[error("State Error - {0}")]
        State(Box<dyn std::error::Error + Send>),
        #[error("Failed to acquire a semaphore permit")]
        SemaphoreClosed(#[from] AcquireError),
    }

    /// Continously fetches signatures from RPC and pipe them over a channel to
    /// further processing.
    ///
    /// # Cancelation Safety
    ///
    /// This function is cancel safe. All lost work can be recovered as the
    /// task's savepoint is sourced from the persistence layer, which
    /// remains unchanged in this context.
    #[tracing::instrument(name = "signature scanner", skip_all, err)]
    pub async fn run<S>(
        address: Pubkey,
        url: Url,
        state: S,
        signature_sender: Sender<Signature>,
        period: Duration,
        cancellation_token: CancellationToken,
        semaphore: Arc<Semaphore>,
    ) -> Result<Never, SignatureScannerError>
    where
        S: State<Signature> + Clone,
    {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let rpc_client = Arc::new(RpcClient::new(url.to_string()));

        loop {
            // Greedly ask for all available permits from this semaphore, to ensure this
            // task will not concur with `transaction_retriever::fetch` tasks.
            let wait_for_all_permits = semaphore
                .clone()
                .acquire_many_owned(MAX_CONCURRENT_RPC_REQUESTS as u32);
            let all_permits = tokio::select! {
                all_permits = wait_for_all_permits => { all_permits? }
                _ = cancellation_token.cancelled() => { Err(SignatureScannerError::Cancelled)? }
            };
            trace!("acquired all semaphore permits (exclusive access)");

            // Now that we have all permits, scan for signatures while waiting for the
            // cancelation signal.
            let future = collect_and_process_signatures(
                address,
                rpc_client.clone(),
                state.clone(),
                signature_sender.clone(),
            );
            tokio::select! {
                res = future => { res }
                _ = cancellation_token.cancelled() => { Err(SignatureScannerError::Cancelled) }
            }?;

            // Give back all permits to the semaphore.
            drop(all_permits);

            // Give some time for downstream futures to acquire permits from this semaphore.
            trace!("sleeping");
            interval.tick().await;
        }
    }

    /// Calls Solana RPC after relevant transaction signatures and send results
    /// over a channel.
    #[tracing::instrument(skip_all, err)]
    async fn collect_and_process_signatures<S>(
        address: Pubkey,
        rpc_client: Arc<RpcClient>,
        state: S,
        signature_sender: Sender<Signature>,
    ) -> Result<(), SignatureScannerError>
    where
        S: State<Signature>,
    {
        // Collect Signatures until exhaustion
        let last_known_signature: Option<Signature> = state
            .get()
            .await
            .map_err(|state_error| SignatureScannerError::State(Box::new(state_error)))?;

        let collected_signatures =
            fetch_signatures_until_exhaustion(rpc_client.clone(), address, last_known_signature)
                .await?;

        // Iterate backwards so oldest signatures are picked up first on the other end.
        for signature in collected_signatures.into_iter().rev() {
            signature_sender.send(signature).await?;
        }
        Ok(())
    }

    /// Fetches all Solana transaction signatures for an address until a
    /// specified signature is reached or no more transactions are
    /// available.
    #[tracing::instrument(skip(rpc_client, address), err)]
    async fn fetch_signatures_until_exhaustion(
        rpc_client: Arc<RpcClient>,
        address: Pubkey,
        until: Option<Signature>,
    ) -> Result<Vec<Signature>, SignatureScannerError> {
        /// This is the max number of signatures returned by the Solana RPC. It
        /// is used as an indicator to tell if we need to continue
        /// querying the RPC for more signatures.
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
                .get_signatures_for_address_with_config(&address, config(last_visited))
                .await?;

            // Get the last (oldest) signature on this batch or break if it is empty
            let Some(oldest) = batch.last() else { break };

            // Set up following calls to start from the point this one had left
            last_visited = Some(Signature::from_str(&oldest.signature)?);

            let batch_size = batch.len();
            collected_signatures.extend(batch.into_iter());

            // If the results are less than the limit, then it means we have all the
            // signatures we need.
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

/// Functions to resolve transaction signatures into full transactions, with
/// metadata.
pub mod transaction_retriever {
    use solana_client::client_error::ClientError;
    use solana_transaction_status::option_serializer::OptionSerializer;
    use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
    use tokio::sync::mpsc::error::SendError;
    use tokio::sync::AcquireError;
    use tracing::debug;
    use TransactionScannerMessage::Message;

    use super::*;

    #[derive(Error, Debug)]
    pub enum TransactionRetrieverError {
        #[error("Transaction Retriever received the cancellation signal")]
        Cancelled,
        #[error("Signature receiver channel closed unexpectedly")]
        SignatureReceiverChannelClosed,
        #[error("Failed to decode solana transaction: {signature}")]
        TransactionDecode { signature: Signature },
        #[error(transparent)]
        NonFatal(#[from] NonFatalError),
        /// This variant's value needs to be boxed to prevent a recursive type
        /// definition, since this error is also part of
        /// [`TransactionScannerMessage`].
        #[error("Failed to send proceesed transaction for event analysis: {0}")]
        SendTransactionError(#[from] Box<SendError<TransactionScannerMessage>>),
        #[error(transparent)]
        SolanaClient(#[from] ClientError),
        #[error("Failed to acquire a semaphore permit")]
        SemaphoreClosed(#[from] AcquireError),
    }

    /// Errors that shouldn't halt the Sentinel.
    #[derive(Error, Debug)]
    pub enum NonFatalError {
        #[error("Got wrong signature from RPC. Expected: {expected}, received: {received}")]
        WrongTransactionReceived {
            expected: Signature,
            received: Signature,
        },
        #[error("Got a transaction without meta attribute: {signature}")]
        TransactionWithoutMeta { signature: Signature },
        #[error("Got a transaction without logs: {signature}")]
        TransactionWithoutLogs { signature: Signature },
    }

    /// Asynchronously processes incoming transaction signatures by spawning
    /// Tokio tasks to retrieve full transaction details.
    ///
    /// Tasks wait acquiring a semaphore permit before reaching the Solana RPC
    /// endpoint.
    ///
    /// Successfully fetched transactions are sent through a channel for
    /// further processing.
    ///
    /// # Cancellation Safety
    ///
    /// This function is cancel safe. All lost work can be recovered as the
    /// task's savepoint is sourced from the persistence layer, which
    /// remains unchanged in this context.
    #[tracing::instrument(name = "transaction-retriever", skip_all)]
    pub async fn run(
        url: Url,
        mut signature_receiver: Receiver<Signature>,
        transaction_sender: Sender<TransactionScannerMessage>,
        cancellation_token: CancellationToken,
        semaphore: Arc<Semaphore>,
    ) -> TransactionRetrieverError {
        let rpc_client = Arc::new(RpcClient::new(url.to_string()));
        let build_future = |signature: Signature| {
            fetch_with_permit(
                signature,
                rpc_client.clone(),
                semaphore.clone(),
                cancellation_token.clone(),
            )
        };

        loop {
            tokio::select! {
                _ = cancellation_token.cancelled() => {
                    trace!("cancelled");
                    return TransactionRetrieverError::Cancelled
                }
                optional_message = signature_receiver.recv() => {
                    let Some(signature) =  optional_message else {
                        return TransactionRetrieverError::SignatureReceiverChannelClosed;
                    };
                    trace!(%signature);
                    let future = build_future(signature);
                    let task = tokio::task::spawn(future);
                    if let Err(err) = transaction_sender.send(Message(task)).await {
                        return Box::new(err).into();
                    }
                }
            }
        }
    }

    /// Fetches a Solana transaction by calling the `getTransactionWithConfig`
    /// RPC method with its signature and decoding the result.
    #[tracing::instrument(skip(rpc_client))]
    async fn fetch(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
    ) -> Result<SolanaTransaction, TransactionRetrieverError> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let EncodedConfirmedTransactionWithStatusMeta {
            block_time,
            slot,
            transaction: transaction_with_meta,
        } = rpc_client
            .get_transaction_with_config(&signature, config)
            .await?;

        let decoded_transaction = transaction_with_meta
            .transaction
            .decode()
            .ok_or_else(|| TransactionRetrieverError::TransactionDecode { signature })?;

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

        let transaction = SolanaTransaction {
            signature,
            logs,
            block_time,
            slot,
        };

        debug!(
            signature = %transaction.signature,
            block_time = ?transaction.block_time,
            slot = %transaction.slot,
            "found solana transaction"
        );

        Ok(transaction)
    }

    /// Fetches a Solana transaction for the given signature once a semaphore
    /// permit is acquired.
    ///
    /// # Cancellation Safety
    ///
    /// This function is cancel safe. It will return without reaching the Solana
    /// RPC endpoint if a cancellation signal is received while waiting for
    /// a semaphore permit.
    async fn fetch_with_permit(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
        semaphore: Arc<Semaphore>,
        cancellation_token: CancellationToken,
    ) -> Result<SolanaTransaction, TransactionRetrieverError> {
        tokio::select! {
            _ = cancellation_token.cancelled() => {
                Err(TransactionRetrieverError::Cancelled)
            }

            permit = semaphore.acquire_owned() => {
                let _permit = permit?;
                fetch(signature, rpc_client).await
            }
        }
    }
}
