use crate::amplifier_api::{
    self, amplifier_client::AmplifierClient, VerifyRequest, VerifyResponse,
};
use crate::{state::State, transports::SolanaToAxelarMessage};
use dashmap::DashMap;
use futures::{FutureExt, TryFutureExt};
use futures_concurrency::future::Race;
use solana_sdk::signature::Signature;
use std::{convert::Infallible as Never, sync::Arc};
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use url::Url;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error(transparent)]
    TonicTransportError(#[from] tonic::transport::Error),
    #[error("Failed to subscribe to the Amplifier API verification stream: {0}")]
    Subscription(tonic::Status),
    #[error("Failed to obtain a response from the Amplifier API verification stream: {0}")]
    StreamIngestion(tonic::Status),
    #[error("The Amplifier API verification stream has been closed unexpectedly")]
    StreamClosed,
    #[error("The Amplifier API verification stream validated and returned an unknown message ID")]
    UnknownMessageId(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Cancellation signal received")]
    Cancelled,
}

/// Axelar Verifier
///
/// Listens for Solana Gateway events and registers them in the Amplifier API.
pub struct AxelarVerifier {
    endpoint: Url,
    receiver: mpsc::Receiver<SolanaToAxelarMessage>,
    state: State,
    cancellation_token: CancellationToken,
}

impl AxelarVerifier {
    pub fn new(
        endpoint: Url,
        receiver: mpsc::Receiver<SolanaToAxelarMessage>,
        state: State,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            endpoint,
            receiver,
            state,
            cancellation_token,
        }
    }

    /// Runs the Axelar Verifier until an error or cancellation occurs.
    ///
    /// Recovery is not possible because the receiving channel end being consumed by the
    /// ReceiverStream to connect with the Amplifier API.
    #[tracing::instrument(name = "axelar-verifier", skip_all)]
    pub async fn run(self) {
        let error = self.work().await.unwrap_err();
        error!(%error, "Axelar Verifier terminated");
    }

    /// Sends incoming [`axl_rpc::Message`] values to the Amplifier API for verification.
    async fn work(self) -> Result<Never, VerifierError> {
        let mut client = AmplifierClient::connect(self.endpoint.to_string()).await?;

        // Track pending messages and their signatures.
        // Signatures are registered by their Message ID, to be later retrieved (and removed) to update the Relayer state.
        let pending = Arc::new(DashMap::new());
        let pending_incoming = pending.clone();

        // Wrap the Message channel into a stream
        let message_stream = ReceiverStream::new(self.receiver).filter_map(move |message| {
            let SolanaToAxelarMessage { message, signature } = message;

            // Filter duplicated incoming messages
            match pending_incoming.insert(message.id.clone(), signature) {
                None => {
                    info!(msg_id = message.id, %signature, "submitting message for verification");
                    Some(VerifyRequest {
                        message: Some(message),
                    })
                }
                Some(duplicated) if duplicated == signature => {
                    warn!( msg_id = message.id, %signature, "ignoring duplicated message");
                    None
                }
                Some(previous) => {
                    error!(
                        msg_id = message.id,
                        previous_signature = %previous,
                        new_signature = %signature,
                        "got a different signature for the same message"
                    );
                    None
                }
            }
        });

        // Connect the Message stream to the verification stream
        let mut verification_stream = client
            .verify(message_stream)
            .await
            .map_err(VerifierError::Subscription)?
            .into_inner();

        // Listen for new verification responses until an error occurs or a shutdown signal is received.
        loop {
            let state = self.state.clone();
            let pending = pending.clone();

            let cancellation = self
                .cancellation_token
                .cancelled()
                .map(|()| Err(VerifierError::Cancelled));

            let message = verification_stream
                .message()
                .map_err(VerifierError::StreamIngestion)
                .and_then(|message| async move {
                    let response = message.ok_or(VerifierError::StreamClosed)?;
                    process_response(response, pending, state).await
                });

            (cancellation, message).race().await?;
        }
    }
}

#[tracing::instrument(skip_all)]
async fn process_response(
    response: VerifyResponse,
    pending: Arc<DashMap<String, Signature>>,
    state: State,
) -> Result<(), VerifierError> {
    let VerifyResponse { message, error } = response;

    match (message, error) {
        // Success case
        (Some(message), None) => {
            // Retrieve Solana Signature associated with this message to update the Relayer state.
            let Some((_, signature)) = pending.remove(&message.id) else {
                return Err(VerifierError::UnknownMessageId(message.id));
            };
            info!(msg_id = message.id, %signature, "message verified");
            state.update_solana_transaction(signature).await?;
        }
        // Error cases
        (Some(message), Some(error)) => {
            let amplifier_api::Error { error, error_code } = error;
            // Remove the message's signature from the pending registry but don't mark it
            // as completed. This is not ideal because future messages can adavnce the
            // Relayer state, effectively skipping this one.
            let Some((_, signature)) = pending.remove(&message.id) else {
                return Err(VerifierError::UnknownMessageId(message.id));
            };
            error!(msg_id = message.id, %signature, %error, %error_code, "failed to verify message");
        }
        (None, Some(error)) => {
            let amplifier_api::Error { error, error_code } = error;
            error!(msg_id = "missing", %error, %error_code, "failed to verify message");
        }
        // No-op case
        (None, None) => warn!("Got an empty response from Amplifier API"),
    };
    Ok(())
}
