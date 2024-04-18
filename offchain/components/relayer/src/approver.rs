use thiserror::Error;
use tokio::sync::mpsc::Sender as TokioSender;
use tokio_util::sync::CancellationToken;
use tonic::Status;
use tracing::{error, info};
use url::Url;

use crate::amplifier_api::amplifier_client::AmplifierClient;
use crate::amplifier_api::{SubscribeToApprovalsRequest, SubscribeToApprovalsResponse};
use crate::state::State;

type Sender = TokioSender<SubscribeToApprovalsResponse>;

use crate::config::SOLANA_CHAIN_NAME;

#[derive(Debug, Error)]
pub enum ApproverError {
    #[error(transparent)]
    TonicTransportError(#[from] tonic::transport::Error),
    #[error("Failed to subscribe for approvals from Axelar - {0}")]
    SubForApprovals(tonic::Status),
    #[error("Failed to pull approvals from Axelar")]
    ApprovalsPull(tonic::Status),
    #[error("State error - {0}")]
    State(#[from] sqlx::Error),
    #[error("Sender channel is closed")]
    SenderChannelClosed,
    #[error("Approvals stream was closed")]
    ApprovalsStreamClosed,
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
    state: State,
    cancellation_token: CancellationToken,
}

impl AxelarApprover {
    /// Create a new sentinel, watching for messages coming from Axelar
    pub fn new(
        rpc_url: Url,
        includer_sender: Sender,
        state: State,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            rpc_url,
            includer_sender,
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
        let start_height = self.state.get_axelar_block_height().await?;

        let mut client = AmplifierClient::connect(self.rpc_url.to_string()).await?;

        // Init the stream of proofs coming from the Amplifier API
        let mut stream = client
            .subscribe_to_approvals(SubscribeToApprovalsRequest {
                chains: vec![SOLANA_CHAIN_NAME.into()],
                start_height: Some(start_height as u64),
            })
            .await
            .map_err(ApproverError::SubForApprovals)?
            .into_inner();

        // Initiate the processing loop. It will pull messages from Axelar side and
        // send them to the Solana includer component. In case any stream is
        // closed, the activity will be logged and the general process cancellation
        // signal will be sent.
        loop {
            tokio::select! {
                axl_proof_res = stream.message() => self.process_message(axl_proof_res).await?,
                _ = self.cancellation_token.cancelled() => {
                    break;
                }
            }
        }
        Ok(()) // Process was gracefully shutdown.
    }

    async fn process_message(
        &self,
        axl_proof_res: Result<Option<SubscribeToApprovalsResponse>, Status>,
    ) -> Result<(), ApproverError> {
        let Some(message) = axl_proof_res.map_err(ApproverError::ApprovalsPull)? else {
            return Err(ApproverError::ApprovalsStreamClosed);
        };

        let block_height = message.block_height;
        self.includer_sender
            .send(message)
            .await
            .map_err(|_| ApproverError::SenderChannelClosed)?;
        // Below we provide a clue that messages are being processed.
        info!(%block_height, "Sent approver => includer message");
        Ok(())
    }
}
