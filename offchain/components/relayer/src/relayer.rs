use amplifier_api::axl_rpc;
use anyhow::{Context, Result};
use tokio::{
    sync::mpsc::{self, Receiver, Sender},
    task::JoinSet,
};
use tracing::{error, info};

use crate::{config, sentinel::SolanaSentinel, state::State};

pub struct Relayer {
    axelar_to_solana: Option<AxelarToSolanaHandler>,
    solana_to_axelar: Option<SolanaToAxelarHandler>,
}

impl Relayer {
    pub async fn from_config(config: config::Config) -> Result<Self> {
        config.validate()?;

        let config::Config {
            axelar_to_solana,
            solana_to_axelar,
            database,
        } = config;

        let state = State::from_url(database.url)
            .await
            .context("Failed to connect to Relayer database")?;

        state.migrate().await?;

        Ok(Relayer {
            axelar_to_solana: axelar_to_solana.map(|config| {
                AxelarToSolanaHandler::new(config.approver, config.includer, state.clone())
            }),
            solana_to_axelar: solana_to_axelar
                .map(|config| SolanaToAxelarHandler::new(config.sentinel, config.verifier, state)),
        })
    }

    pub async fn run(self) {
        match (self.axelar_to_solana, self.solana_to_axelar) {
            (Some(axelar_to_solana), Some(solana_to_axelar)) => {
                tokio::join!(solana_to_axelar.run(), axelar_to_solana.run());
            }
            (Some(axelar_to_solana), None) => {
                axelar_to_solana.run().await;
            }
            (None, Some(solana_to_axelar)) => {
                solana_to_axelar.run().await;
            }
            (None, None) => {
                error!("Relayer was set to run without configured transports.")
            }
        };
        info!("Relayer is shutting down");
    }
}

//
// Transport types
//

// TODO: Transports look very similar. We can make a single type that is generic over its actors.

struct AxelarToSolanaHandler {
    approver: config::AxelarApprover,
    includer: config::SolanaIncluder,
    state: State,
}

impl AxelarToSolanaHandler {
    fn new(
        approver: config::AxelarApprover,
        includer: config::SolanaIncluder,
        state: State,
    ) -> Self {
        Self {
            approver,
            includer,
            state,
        }
    }

    async fn run(self) {
        loop {
            info!("Starting Axelar to Solana transport");
            let (approver, includer) =
                AxelarToSolanaHandler::setup_actors(&self.approver, &self.includer);

            let mut set = JoinSet::new();
            set.spawn(approver.run());
            set.spawn(includer.run());

            while let Some(_) = set.join_next().await {
                error!("Axelar to Solana transport failed. Restarting.");
                set.abort_all();
            }
        }
    }

    fn setup_actors(
        _approver_config: &config::AxelarApprover,
        _includer_config: &config::SolanaIncluder,
    ) -> (ApproverActor, IncluderActor) {
        // TODO: use config to properly initialize actors
        let (sender, receiver) = mpsc::channel(500); // FIXME: magic number
        let approver = ApproverActor { sender };
        let includer = IncluderActor { receiver };
        (approver, includer)
    }
}

struct SolanaToAxelarHandler {
    sentinel: config::SolanaSentinel,
    verifier: config::AxelarVerifier,
    state: State,
}

impl SolanaToAxelarHandler {
    fn new(
        sentinel: config::SolanaSentinel,
        verifier: config::AxelarVerifier,
        state: State,
    ) -> Self {
        Self {
            sentinel,
            verifier,
            state,
        }
    }

    /// Runs this transport forever, restarting whenever any of the internal actors fail for any reason.
    async fn run(self) {
        loop {
            info!("Starting Solana to Axelar transport");
            let (sentinel, verifier) = SolanaToAxelarHandler::setup_actors(
                &self.sentinel,
                &self.verifier,
                self.state.clone(),
            );

            let mut set = JoinSet::new();
            set.spawn(sentinel.run());
            set.spawn(verifier.run());

            while let Some(_) = set.join_next().await {
                error!("Solana to Axelar transport failed. Restarting.");
                set.abort_all();
            }
        }
    }

    fn setup_actors(
        sentinel_config: &config::SolanaSentinel,
        _verifier_config: &config::AxelarVerifier,
        state: State,
    ) -> (SolanaSentinel, VerifierActor) {
        // TODO: use config to properly initialize actors
        let (sender, receiver) = mpsc::channel::<axl_rpc::Message>(500); // FIXME: magic number

        // Solana Sentinel
        let config::SolanaSentinel {
            gateway_address,
            rpc,
        } = sentinel_config;
        let sentinel = SolanaSentinel::new(*gateway_address, rpc.clone(), sender, state.clone());

        // Axelar Verifier
        let verifier = VerifierActor { receiver, state };
        (sentinel, verifier)
    }
}

//
// Actor Placheholder Types
//

// TODO: Use the real worker types already defined in this crate.

struct VerifierActor {
    receiver: Receiver<axl_rpc::Message>,
    state: State,
}

impl VerifierActor {
    async fn run(self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}

struct ApproverActor {
    sender: Sender<()>,
}

impl ApproverActor {
    async fn run(self) {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

struct IncluderActor {
    receiver: Receiver<()>,
}

impl IncluderActor {
    async fn run(self) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
