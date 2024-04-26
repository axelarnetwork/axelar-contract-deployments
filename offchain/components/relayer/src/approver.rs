use axelar_executable::axelar_message_primitives::command::{decode, DecodeError};
use axelar_executable::axelar_message_primitives::{DataPayload, PayloadError};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use solana_sdk::pubkey::Pubkey;
use thiserror::Error;
use tokio::sync::mpsc::Sender as TokioSender;
use tokio_util::sync::CancellationToken;
use tonic::transport::Channel;
use tracing::{error, info};
use url::Url;

use crate::amplifier_api::amplifier_client::AmplifierClient;
use crate::amplifier_api::{
    GetPayloadRequest, SubscribeToApprovalsRequest, SubscribeToApprovalsResponse,
};
use crate::state::State;

type Sender = TokioSender<SubscribeToApprovalsResponse>;

use crate::config::SOLANA_CHAIN_NAME;

#[derive(Debug, Error)]
pub enum ApproverError {
    #[error(transparent)]
    TonicTransportError(#[from] tonic::transport::Error),
    #[error("Failed to subscribe for approvals from Axelar: {0}")]
    SubForApprovals(#[source] tonic::Status),
    #[error("Failed to pull approvals from Axelar: {0}")]
    ApprovalsPull(#[source] tonic::Status),
    #[error("Failed to fetch message full payload: {0}")]
    GetPayload(#[source] tonic::Status),
    #[error("State error: {0}")]
    State(#[from] sqlx::Error),
    #[error("Sender channel is closed")]
    SenderChannelClosed,
    #[error("Approvals stream was closed")]
    ApprovalsStreamClosed,
    #[error("Failed to decode command batch: {0}")]
    DecodeCommandBatch(#[from] DecodeError),
    #[error("Failed to decode message payload: {0}")]
    DecodePayload(#[from] PayloadError),
    #[error("Found Relayer own address inside a message")]
    RelayerAddressReference {
        payload_hash: [u8; 32],
        command_batch_hash: [u8; 32],
    },
}

/// Listens for approved messages (signed proofs) coming from the Axelar
/// blockchain.
///
/// Those will be payloads sent from other blockchains,
/// which pass through axelar and are sent to Solana.
#[allow(dead_code)]
pub struct AxelarApprover {
    rpc_url: Url,
    includer_sender: Sender,
    relayer_account: Pubkey,
    state: State,
    cancellation_token: CancellationToken,
}

impl AxelarApprover {
    /// Create a new sentinel, watching for messages coming from Axelar
    pub fn new(
        rpc_url: Url,
        includer_sender: Sender,
        relayer_account: Pubkey,
        state: State,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            rpc_url,
            includer_sender,
            relayer_account,
            state,
            cancellation_token,
        }
    }

    #[tracing::instrument(skip(self), name = "Axelar approver component")]
    pub async fn run(mut self) {
        if let Err(err) = self.work().await {
            self.cancellation_token.cancel(); // send shutdown signal to other components.
            error!(%err, "Axelar Approver terminated");
        } else {
            info!("Axelar Approver was gracefully shut down");
        }
    }

    async fn work(&mut self) -> Result<(), ApproverError> {
        // Connect to the Amplifier API
        let mut client = AmplifierClient::connect(self.rpc_url.to_string()).await?;

        // Get the latest processed block.
        // TODO: We might want to artificially subtract from the obtained block height
        // as to cover for possible partially processed blocks.
        let start_height = self.state.get_axelar_block_height().await?;

        // Connect to the Amplifier API for an approval stream.
        let mut stream = client
            .subscribe_to_approvals(SubscribeToApprovalsRequest {
                chains: vec![SOLANA_CHAIN_NAME.into()],
                start_height: Some(start_height as u64),
            })
            .await
            .map_err(ApproverError::SubForApprovals)?
            .into_inner();

        // Used to validate incoming messages concurrently.
        let mut validation_futures = FuturesUnordered::new();

        // Concurrently listens for incoming approvals and validate them, until either
        // the cancelation signal is sent or an unreccverable error appears.
        loop {
            tokio::select! {
                // Listen for approvals and send them to validation
                message = stream.message() => {
                    let unvalidated_approval = message
                        .map_err(ApproverError::ApprovalsPull)?
                        .ok_or(ApproverError::ApprovalsStreamClosed)?;
                    info!("Approval received");
                    validation_futures.push(self.validate(client.clone(), unvalidated_approval));
                }
                // Send validated approvals downstream.
                Some(validation_result) = validation_futures.next() => {
                    match validation_result {
                        Ok(validated_approval) => {
                            info!("Validated incoming approval");
                            self.includer_sender
                                .send(validated_approval)
                                .await
                                .map_err(|_| ApproverError::SenderChannelClosed)?
                        },
                        Err(validation_error) => {
                            // Log the error without halting the Approver
                            error!(%validation_error);
                        },
                    }
                }
                // Listen for the cancellation signal
                _ = self.cancellation_token.cancelled() => {
                    break;
                }
            }
        }
        Ok(()) // Process was gracefully shutdown.
    }

    #[tracing::instrument(skip_all, err)]
    async fn validate(
        &self,
        client: AmplifierClient<Channel>,
        message: SubscribeToApprovalsResponse,
    ) -> Result<SubscribeToApprovalsResponse, ApproverError> {
        let (_proof, command_batch, command_batch_hash) = decode(&message.execute_data)?;
        // XXX: The Approver could also validate the proof at this point.

        // Fetch payloads concurrently.
        let mut future_payloads: FuturesUnordered<_> = command_batch
            .commands
            .iter()
            .filter_map(|cmd| cmd.payload_hash())
            .map(|hash| get_payload(client.clone(), hash))
            .collect();

        // Return an error if any decoded payload references the Relayer address in
        // their instruction definitions.
        while let Some(response) = future_payloads.next().await {
            let PayloadAndHash { payload, hash } = response?;
            let payload = DataPayload::decode(&payload)?;
            if payload
                .account_meta()
                .iter()
                .any(|meta| meta.pubkey == self.relayer_account)
            {
                return Err(ApproverError::RelayerAddressReference {
                    payload_hash: hash,
                    command_batch_hash,
                });
            }
        }
        Ok(message)
    }
}

/// Retrieves the full payload data for a given payload hash.
///
/// Since this function is designed to run within a `FuturesUnordered` context,
/// we can't directly associate the returned payload with the input hash.
/// Therefore, its return type holds both the payload and the original hash.
#[tracing::instrument(skip(client), err)]
async fn get_payload(
    mut client: AmplifierClient<Channel>,
    hash: [u8; 32],
) -> Result<PayloadAndHash, ApproverError> {
    let request = GetPayloadRequest { hash: hash.into() };
    client
        .get_payload(request)
        .await
        .map(|response| response.into_inner().payload)
        .map(|payload| PayloadAndHash { payload, hash })
        .map_err(ApproverError::GetPayload)
    // TODO: Re-hash payload and check if it matches with the original hash.
}

struct PayloadAndHash {
    payload: Vec<u8>,
    hash: [u8; 32],
}
