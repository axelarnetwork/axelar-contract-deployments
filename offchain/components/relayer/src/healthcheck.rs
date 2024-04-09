//! Health check server.

use std::future::Future;
use std::io;
use std::net::SocketAddr;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tracing::{error, info};

/// Guard to a running health check server.
///
/// Gracefully shuts down the server when dropped.
pub struct HealthCheckServerGuard {
    /// Sender half of the shutdown channel.
    ///
    /// This must be wrapped in an `Option` beacause it is used inside `Drop`,
    /// which exposes only `&mut self`.
    shutdown: Option<oneshot::Sender<()>>,
}

impl Drop for HealthCheckServerGuard {
    /// Sends the shutdown signal on drop.
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            match shutdown.send(()) {
                Ok(_) => info!("shutting down health check server"),
                Err(_) => error!("health check server shutdown failed"),
            };
        }
    }
}

/// Starts the health check server in a dedicated Tokio task.
///
/// Returns a `HealthCheckServerGuard` handle that gracefully shuts down the
/// server when dropped.
pub async fn start(address: SocketAddr) -> Result<HealthCheckServerGuard, io::Error> {
    let listener = TcpListener::bind(address).await?;
    start_with_tcp_listener(listener).await
}

/// Starts the health check server in a dedicated tokio task.
///
/// Splitting this function in two was necessary because unit tests don't know
/// which port to connect to beforehand, so they can only pass a [`TcpListener`]
/// as an argument.
async fn start_with_tcp_listener(
    listener: TcpListener,
) -> Result<HealthCheckServerGuard, io::Error> {
    // Set up graceful signal.
    // The receiving end will be notified when the guard is dropped.
    let (sender, receiver) = oneshot::channel::<()>();
    let shutdown_signal = async move {
        let _ = receiver.await;
    };

    // Run the health check server
    tokio::spawn(run_server(listener, shutdown_signal));

    Ok(HealthCheckServerGuard {
        shutdown: Some(sender),
    })
}

/// Runs an HTTP server until the shutdown signal is sent.
async fn run_server<F>(tcp_listener: TcpListener, shutdown_signal: F) -> anyhow::Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let router = Router::new().route("/status", get(|| async { StatusCode::OK }));
    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;
    use std::time::Duration;

    use figment::providers::{Env, Serialized};
    use figment::Figment;
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct HealthCheckServerTestConfig {
        warmup_millis: u64,
        cooldown_millis: u64,
    }

    impl Default for HealthCheckServerTestConfig {
        fn default() -> Self {
            Self {
                warmup_millis: 100,
                cooldown_millis: 100,
            }
        }
    }

    fn test_config() -> &'static HealthCheckServerTestConfig {
        static CONFIG: OnceLock<HealthCheckServerTestConfig> = OnceLock::new();
        CONFIG.get_or_init(|| {
            Figment::from(Serialized::defaults(HealthCheckServerTestConfig::default()))
                .merge(Env::prefixed("RELAYER_TEST_HEALTHCHECK_"))
                .extract()
                .expect("failed to parse Relayer test configuration")
        })
    }

    async fn wait(millis: u64) {
        tokio::time::sleep(Duration::from_millis(millis)).await;
    }

    async fn warmup() {
        wait(test_config().warmup_millis).await;
    }

    async fn cooldown() {
        wait(test_config().cooldown_millis).await;
    }

    #[tokio::test]
    async fn server_lifecycle() -> anyhow::Result<()> {
        let host = "127.0.0.1";
        // Bind to localhost at the port 0, which will let the OS assign an available
        // port to us.
        let listener = TcpListener::bind(format!("{host}:0")).await?;
        // Retrieve the local address assigned to us by the OS.
        let local_address = listener.local_addr()?;

        // Start server and let it warm up a bit.
        let server = start_with_tcp_listener(listener).await?;
        warmup().await;

        let client = reqwest::Client::new();
        let address = format!("http://{local_address}/status");
        let resp = client.get(&address).send().await?;
        assert_eq!(resp.status(), reqwest::StatusCode::OK);

        // Other paths return 404
        for bad_path in ["/", "/bad", "/stat", "/status_"] {
            let bad_address = format!("http://{local_address}/{bad_path}");
            let bad_resp = client.get(&bad_address).send().await?;
            assert_eq!(bad_resp.status(), reqwest::StatusCode::NOT_FOUND);
        }

        // Shut down the server and let it cool down.
        drop(server);
        cooldown().await;

        match client.get(&address).send().await {
            Ok(_) => panic!("health check server should be closed by now"),
            Err(error) => assert!(error.is_connect()),
        };

        Ok(())
    }
}
