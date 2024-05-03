use std::convert::Infallible as Never;
use std::net::SocketAddr;

use anyhow::{anyhow, Context, Result};
use futures_util::FutureExt;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{timeout, Duration};
use tokio::{join, pin, select};
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

use crate::amplifier_api::SubscribeToApprovalsResponse;
use crate::approver::AxelarApprover;
use crate::includer::SolanaIncluder;
use crate::sentinel::SolanaSentinel;
use crate::state::interface::State as StateInterface;
use crate::state::State;
use crate::transports::SolanaToAxelarMessage;
use crate::verifier::AxelarVerifier;
use crate::{config, healthcheck};

pub struct Relayer {
    axelar_to_solana: Option<AxelarToSolanaHandler>,
    solana_to_axelar: Option<SolanaToAxelarHandler>,
    health_check_server_addr: SocketAddr,
}

impl Relayer {
    pub async fn from_config(config: config::Config) -> Result<Self> {
        config.validate()?;

        let config::Config {
            axelar_to_solana,
            solana_to_axelar,
            database,
            health_check,
            cancellation_timeout,
        } = config;

        let state = State::from_url(database.url)
            .await
            .context("Failed to connect to Relayer database")?;

        state.migrate().await?;

        Ok(Relayer {
            axelar_to_solana: axelar_to_solana.map(|config| {
                AxelarToSolanaHandler::new(
                    config.approver,
                    config.includer,
                    state.clone(),
                    cancellation_timeout,
                )
            }),
            solana_to_axelar: solana_to_axelar.map(|config| {
                SolanaToAxelarHandler::new(
                    config.sentinel,
                    config.verifier,
                    state,
                    cancellation_timeout,
                )
            }),
            health_check_server_addr: health_check.bind_addr,
        })
    }

    pub async fn run(self) -> Result<Never> {
        // Start the health check server.
        let _health_check_server = match healthcheck::start(self.health_check_server_addr).await {
            Ok(guard) => {
                info!(address = %self.health_check_server_addr, "Started health check server.");
                guard
            }
            Err(error) => {
                return Err(error).context("Failed to start the health check server")?;
            }
        };

        match (self.axelar_to_solana, self.solana_to_axelar) {
            (Some(axelar_to_solana), Some(solana_to_axelar)) => {
                let mut set = JoinSet::new();
                set.spawn(solana_to_axelar.run());
                set.spawn(axelar_to_solana.run());

                // Await on both transports indefinitely.
                // Unexpected termination of either transport indicates a non-recoverable error,
                // requiring the Relayer to shut down.
                set.join_next().await;
                error!("One of the transports has terminated unexpectedly");
                return Err(anyhow!("Terminating"));
            }

            (Some(axelar_to_solana), None) => {
                let mut set = JoinSet::new();
                set.spawn(axelar_to_solana.run());
                set.join_next().await;
            }
            (None, Some(solana_to_axelar)) => {
                let mut set = JoinSet::new();
                set.spawn(solana_to_axelar.run());
                set.join_next().await;
            }
            (None, None) => {
                error!("Relayer was set to run without configured transports.")
            }
        };
        info!("Relayer is shutting down");
        Err(anyhow!("Terminating"))
    }
}

//
// Transport types
//

// TODO: Transports look very similar. We can make a single type that is generic
// over its actors.

struct AxelarToSolanaHandler {
    approver: config::AxelarApprover,
    includer: config::SolanaIncluder,
    #[allow(dead_code)]
    state: State,
    cancellation_timeout: Duration,
}

impl AxelarToSolanaHandler {
    fn new(
        approver: config::AxelarApprover,
        includer: config::SolanaIncluder,
        state: State,
        cancellation_timeout: Duration,
    ) -> Self {
        Self {
            approver,
            includer,
            state,
            cancellation_timeout,
        }
    }

    async fn run(self) -> Result<Never> {
        loop {
            info!("Starting Axelar to Solana transport");
            let (approver, includer, cancellation_token) = AxelarToSolanaHandler::setup_actors(
                &self.approver,
                &self.includer,
                self.state.clone(),
            );

            // Fuse the futures to allow polling of the other future when one is waiting for
            // shutdown.
            pin! {
                let approver_future = approver.run().fuse();
                let includer_future = includer.run().fuse();
            }

            // Run the Approver and Includer concurrently, and wait for the first one to
            // fail.
            select! {
                _ = &mut approver_future => error!("Axelar Approver has failed"),
                _ = &mut includer_future => error!("Solana Includer has failed")
            }

            // Trigger cancellation and wait for both actors to gracefully shut down
            cancellation_token.cancel();
            tracing::debug!(
                timeout_seconds = self.cancellation_timeout.as_secs(),
                "Waiting for components to gracefully shutdown"
            );
            let _ = join!(
                timeout(self.cancellation_timeout, approver_future),
                timeout(self.cancellation_timeout, includer_future)
            );
            tracing::warn!("Restarting Axelar to Solana transport")
        }
    }

    fn setup_actors(
        approver_config: &config::AxelarApprover,
        includer_config: &config::SolanaIncluder,
        state: State,
    ) -> (AxelarApprover, SolanaIncluder, CancellationToken) {
        // TODO: use config to properly initialize actors
        let (sender, receiver) = mpsc::channel::<SubscribeToApprovalsResponse>(500); // FIXME: magic number

        // This is the root cancellation token for this transport session.
        // It is also used to derive child tokens for each subcomponent, so they can
        // manage their own cancellation schedules. The root token will be
        // cancelled if any subcomponent returns early.
        let transport_cancelation_token = CancellationToken::new();
        let approver_cancelation_token = transport_cancelation_token.child_token();
        let includer_cancelation_token = transport_cancelation_token.child_token();

        let approver = {
            let config::AxelarApprover {
                rpc,
                solana_chain_name,
            } = approver_config;

            // Derive account from secret key
            let relayer_account = includer_config.keypair.pubkey();

            AxelarApprover::new(
                rpc.clone(),
                sender,
                relayer_account,
                state.clone(),
                approver_cancelation_token,
                solana_chain_name.clone(),
            )
        };

        let includer = {
            let config::SolanaIncluder {
                rpc,
                keypair,
                gateway_address,
                gateway_config_address,
            } = includer_config;
            SolanaIncluder::new(
                rpc.clone(),
                keypair.clone(),
                *gateway_address,
                *gateway_config_address,
                receiver,
                state,
                includer_cancelation_token,
            )
        };
        (approver, includer, transport_cancelation_token)
    }
}

struct SolanaToAxelarHandler {
    sentinel: config::SolanaSentinel,
    verifier: config::AxelarVerifier,
    state: State,
    cancellation_timeout: Duration,
}

impl SolanaToAxelarHandler {
    fn new(
        sentinel: config::SolanaSentinel,
        verifier: config::AxelarVerifier,
        state: State,
        cancellation_timeout: Duration,
    ) -> Self {
        Self {
            sentinel,
            verifier,
            state,
            cancellation_timeout,
        }
    }

    /// Runs the Solana-to-Axelar transport indefinitely.
    ///
    /// This function sets up the necessary actors (Sentinel and Verifier) and
    /// runs them concurrently. If either fails for any reason, the entire
    /// transport will be restarted.
    #[tracing::instrument(skip(self), name = "solana-to-axelar-transport")]
    async fn run(self) -> Result<Never> {
        loop {
            info!("Starting Solana to Axelar transport");
            let (sentinel, verifier, cancellation_token) = SolanaToAxelarHandler::setup_actors(
                &self.sentinel,
                &self.verifier,
                self.state.clone(),
            )
            .await?;

            // Fuse the futures to allow polling of the other future when one is waiting for
            // shutdown.
            pin! {
                let sentinel_future = sentinel.run().fuse();
                let verifier_future = verifier.run().fuse();
            }

            // Run the Sentinel and Verifier concurrently, and wait for the first one to
            // fail.
            select! {
                _ = &mut sentinel_future => error!("Solana Sentinel has failed"),
                _ = &mut verifier_future => error!("Axelar Verifier has failed")
            }

            // Trigger cancellation and wait for both actors to gracefully shut down, up to
            // `TIMEOUT`.
            cancellation_token.cancel();
            tracing::debug!(
                timeout_seconds = self.cancellation_timeout.as_secs(),
                "Waiting for components to gracefully shutdown"
            );
            let _ = join!(
                timeout(self.cancellation_timeout, sentinel_future),
                timeout(self.cancellation_timeout, verifier_future)
            );
            tracing::warn!("Restarting Solana-to-Axelar transport")
        }
    }

    async fn setup_actors<S>(
        sentinel_config: &config::SolanaSentinel,
        verifier_config: &config::AxelarVerifier,
        state: S,
    ) -> Result<(SolanaSentinel<S>, AxelarVerifier<S>, CancellationToken)>
    where
        S: StateInterface<Signature> + Clone,
    {
        let (sender, receiver) = mpsc::channel::<SolanaToAxelarMessage>(500); // FIXME: magic number

        // This is the root cancelation token for this transport session.
        // It is also used to derive child tokens for each subcomponent, so they can
        // manage their own cancelation schedules. The root token will be
        // cancelled if any subcomponent returns early.
        let transport_cancelation_token = CancellationToken::new();
        let sentinel_cancelation_token = transport_cancelation_token.child_token();
        let verifier_cancelation_token = transport_cancelation_token.child_token();

        // Solana Sentinel
        let config::SolanaSentinel {
            gateway_address,
            rpc,
            solana_chain_name,
            transaction_scanner,
        } = sentinel_config;
        let sentinel = SolanaSentinel::new(
            *gateway_address,
            rpc.clone(),
            sender,
            state.clone(),
            sentinel_cancelation_token,
            solana_chain_name.clone(),
            *transaction_scanner,
        );

        // Axelar Verifier
        let verifier = AxelarVerifier::new_from_uri(
            verifier_config.rpc.clone(),
            receiver,
            state,
            verifier_cancelation_token,
        )
        .await
        .context("Axelar Verifier failed to connect")?;

        Ok((sentinel, verifier, transport_cancelation_token))
    }
}
