use amplifier_api::axl_rpc::{self, axelar_rpc_client::AxelarRpcClient};
use thiserror::Error;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Failed to subscribe to an Axelar stream of verification messages - {0}")]
    SubForVerificationStreamError(tonic::Status),
    #[error("Failed to fetch verification response from stream - {0}")]
    FetchVerificatoinMsgFromStreamError(tonic::Status),
}

/// Listens for approved messages (signed proofs) coming from the Axelar blockchain
/// using the Amplifier API.
///
/// Those will be messages sent from other blockchains,
/// which pass through axelar and are sent to Solana by the Amplifier API.
#[derive(Clone)]
pub struct Verifier {
    tx: mpsc::UnboundedSender<axl_rpc::Message>,
}

impl Verifier {
    pub fn start(client: AxelarRpcClient<Channel>) -> Self {
        let actor = VerifierActor::new(client);
        let (tx, rx) = mpsc::unbounded_channel();

        tokio::spawn(async move { actor.run(rx).await });

        Self::new(tx)
    }

    fn new(tx: UnboundedSender<axl_rpc::Message>) -> Self {
        Self { tx }
    }

    // TODO: Check if this can be easily fixed
    #[allow(clippy::result_large_err)]
    pub fn verify(
        &self,
        msg: axl_rpc::Message,
    ) -> Result<(), mpsc::error::SendError<axl_rpc::Message>> {
        self.tx.send(msg)
    }
}

struct VerifierActor {
    pub client: AxelarRpcClient<Channel>,
}

impl VerifierActor {
    fn new(client: AxelarRpcClient<Channel>) -> Self {
        Self { client }
    }

    /// Takes GMP messages coming from Solana,
    /// and sends them to the solana gateway for verification via the Amplifier API.
    pub async fn run(
        mut self,
        rx: mpsc::UnboundedReceiver<axl_rpc::Message>,
    ) -> Result<(), VerifierError> {
        // wrap the mpsc channel into a tokio stream
        let gmp_stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx).map(|gmp_msg| {
            axl_rpc::VerifyRequest {
                message: Some(gmp_msg),
            }
        });

        // pass a stream of messages and subscribe to a stream of verificatino responses
        let mut verification_stream = self
            .client
            .verify(gmp_stream)
            .await
            .map_err(VerifierError::SubForVerificationStreamError)?
            .into_inner();
        while let Some(verification_response) = verification_stream
            .message()
            .await
            .map_err(VerifierError::FetchVerificatoinMsgFromStreamError)?
        {
            info!("VERIFICATION RESPONSE {:?}", verification_response);
            if !verification_response.success && verification_response.message.is_some() {
                warn!(
                    "failed to verify msg id {}",
                    verification_response.message.unwrap().id
                );
            }
        }

        Ok(())
    }
}
