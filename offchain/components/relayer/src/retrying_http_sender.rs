use std::time::Duration;

use async_trait::async_trait;
use backoff::future::retry;
use backoff::ExponentialBackoffBuilder;
use serde_json::Value;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_sender::RpcTransportStats;
use solana_rpc_client::http_sender::HttpSender;
use solana_rpc_client::rpc_sender::RpcSender;
use solana_rpc_client_api::client_error::Result as ClientResult;
use solana_rpc_client_api::request::RpcRequest;
use tracing::error;

/// The maximum elapsed time for retrying failed requests.
const TWO_MINUTES: Duration = Duration::from_millis(2 * 60 * 1_000);

/// A wrapper around `HttpSender` that adds retry logic for sending RPC
/// requests.
pub struct RetryingHttpSender(HttpSender);

impl RetryingHttpSender {
    pub fn new(url: String) -> Self {
        let http = HttpSender::new(url);
        RetryingHttpSender(http)
    }

    async fn send(
        &self,
        request: RpcRequest,
        params: &Value,
    ) -> Result<Value, backoff::Error<ClientError>> {
        use ClientErrorKind::*;
        self.0
            .send(request, params.clone())
            .await
            .inspect_err(|error| error!(%error))
            .map_err(|error| match error.kind() {
                // Retry on networking-io related errors
                Io(_) | Reqwest(_) => backoff::Error::transient(error),
                // Fail instantly on other errors
                SerdeJson(_) | RpcError(_) | SigningError(_) | TransactionError(_) | Custom(_) => {
                    backoff::Error::permanent(error)
                }
            })
    }
}

#[async_trait]
impl RpcSender for RetryingHttpSender {
    #[tracing::instrument(skip(self), name = "retrying_http_sender")]
    async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        let strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(TWO_MINUTES))
            .build();
        let operation = || self.send(request, &params);
        retry(strategy, operation).await
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.0.get_transport_stats()
    }

    fn url(&self) -> String {
        self.0.url()
    }
}
