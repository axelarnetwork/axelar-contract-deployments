use std::fmt::Display;
use std::path::PathBuf;

use anyhow::Ok;
use clap::{Parser, Subcommand, ValueEnum};
use url::Url;

use self::cmd::{build_contract, deploy, init_gmp_gateway};
use self::report::Report;

mod cmd;
mod path;
pub mod report;

#[cfg(test)]
mod test_helpers;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Contract {
    GmpGateway,
}

impl Contract {
    /// Provides the predictable output artifact that will be
    /// generated when each contract it's built. This is a helper
    /// method that is normally join'ed() with other base directories.
    fn file(&self) -> PathBuf {
        match self {
            Contract::GmpGateway => PathBuf::from("gmp_gateway.so"),
        }
    }
    /// Provides the local folder name at "solana/programs" each
    /// contract belongs to.
    /// This is a helper method that is normally when it's needed to
    /// i.e "cd" into the contract folder for building it with `cargo-sbf`.
    fn dir(&self) -> PathBuf {
        match self {
            Contract::GmpGateway => PathBuf::from("gateway"),
        }
    }
}

impl Display for Contract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Contract::GmpGateway => write!(f, "gmp-gateway"),
        }
    }
}

/// Xtask is the Axelar Solana workspace CLI that helps
/// both actors, humans and CI to achieve mundane tasks
/// like building, deploying and initializing Solana
/// programs.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub enum Cli {
    /// Build's a contract that is listed in the programs
    /// workspace directory.
    Build {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: Contract,
    },
    /// Deploys the given contract name
    Deploy {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: Contract,
        /// They keypair used to deploy the contract and sign transactions.
        /// If not provided, it will fallback into Solana CLI defaults.
        #[arg(short, long)]
        keypair_path: Option<PathBuf>,
        /// The RPC URL of the target validator. If not provided, it will
        /// fallback into Solana CLI defaults.
        #[arg(short, long)]
        url: Option<Url>,
        /// The websocket URL of the target validator. Normally the same as the
        /// rpc url, but replacing scheme in favour of ws:// . If not
        /// provided, it will fallback into Solana CLI defaults.
        #[arg(short, long)]
        ws_url: Option<Url>,
    },
    Init {
        #[command(subcommand)]
        contract: InitSubcommand,
    },
}

/// Initialize contracts by providing their specific init parameters.
#[derive(Subcommand)]
pub enum InitSubcommand {
    /// Initialize an already deployed gateway contract.
    GmpGateway {
        /// A path that points to a toml file that contains the signers and
        /// their respective weights data. See tests/auth_weighted.toml file
        /// for an example.
        #[arg(short, long)]
        auth_weighted_file: PathBuf,
        /// The RPC URL of the target validator.
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        rpc_url: Option<Url>,
        /// The payer keypair file. This is a file containing the byte slice
        /// serialization of a solana_sdk::signer::keypair::Keypair .
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        payer_kp_path: Option<PathBuf>,
    },
}

impl Cli {
    pub async fn run(&self) -> anyhow::Result<Report> {
        match self {
            Cli::Build { contract } => Ok(Report::Build(build_contract(contract).await?)),
            Cli::Deploy {
                contract,
                keypair_path,
                url,
                ws_url,
            } => deploy(contract, keypair_path, url, ws_url).await,
            Cli::Init { contract } => match contract {
                InitSubcommand::GmpGateway {
                    rpc_url,
                    payer_kp_path,
                    auth_weighted_file: auth_weighted,
                } => init_gmp_gateway(auth_weighted, rpc_url, payer_kp_path).await,
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use std::io::Write;
    use std::thread;
    use std::time::Duration;

    use axelar_message_primitives::command::U256;
    use axelar_message_primitives::Address;
    use borsh::from_slice;
    use gmp_gateway::axelar_auth_weighted::AxelarAuthWeighted;
    use gmp_gateway::state::GatewayConfig;
    use solana_test_validator::{TestValidatorGenesis, UpgradeableProgramInfo};
    use tempfile::NamedTempFile;
    use tests::path::contracts_artifact_dir;
    use tests::test_helpers::build_gateway_contract;

    use super::*;

    #[tokio::test]
    async fn build_actually_works() {
        let args = vec!["xtask", "build", "gmp-gateway"];

        let cli: Cli = Cli::try_parse_from(args).unwrap();

        let result = cli.run().await.unwrap();

        let contract_path = match result {
            Report::Build(report) => report,
            _ => panic!("result not expected."),
        };

        assert!(contract_path.exists())
    }

    #[tokio::test]
    async fn deploy_actually_works() {
        solana_logger::setup_with_default("solana_program_runtime=warn");

        let validator = TestValidatorGenesis::default();

        let (validator, keypair) = validator.start_async().await;

        validator.set_startup_verification_complete_for_tests();

        let mut file = NamedTempFile::new().unwrap();

        write!(file, "{:?}", keypair.to_bytes()).unwrap();

        // Bindings for cmd creation
        let file_path = file.path().to_string_lossy();
        let rpc_url = validator.rpc_url();
        let rpc_pubsub_url = validator.rpc_pubsub_url();

        let args = vec![
            "xtask",
            "deploy",
            "-k",
            &file_path,
            "-u",
            &rpc_url,
            "-w",
            &rpc_pubsub_url,
            "gmp-gateway",
        ];

        let cli: Cli = Cli::try_parse_from(args).unwrap();

        let result = cli.run().await.unwrap();

        let validator_rpc_client = validator.get_async_rpc_client();

        let contract_id = match result {
            Report::Deploy(report) => report,
            _ => panic!("result not expected."),
        };

        let account_info = validator_rpc_client
            .get_account(&contract_id)
            .await
            .unwrap();

        assert!(account_info.executable)
    }

    #[tokio::test]
    async fn initialize_gateway_contract_works() {
        solana_logger::setup_with_default("solana_program_runtime=warn");

        build_gateway_contract();

        // Bring up the validator + the target contract to initialise.
        let mut seed_validator = TestValidatorGenesis::default();
        let program_id = gmp_gateway::id();
        seed_validator.add_upgradeable_programs_with_path(&[UpgradeableProgramInfo {
            program_id,
            loader: solana_sdk::bpf_loader_upgradeable::id(),
            upgrade_authority: program_id,
            program_path: contracts_artifact_dir()
                .unwrap()
                .join(Contract::GmpGateway.file()),
        }]);
        let (validator, keypair) = seed_validator.start_async().await;

        // Save private keypair to temp file for the test
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{:?}", keypair.to_bytes()).unwrap();

        // Prepare cmd
        let rpc_url = validator.rpc_url();
        let payer_kp = file.path().to_string_lossy();

        let args = vec![
            "xtask",
            "init",
            "gmp-gateway",
            "--rpc-url",
            &rpc_url,
            "--payer-kp-path",
            &payer_kp, // We use the already funded keypair.
            "--auth-weighted-file",
            "tests/auth_weighted.toml",
        ];

        // Wait to programs to be consolidated in the validator.
        thread::sleep(Duration::from_millis(15000));

        // Execute CLI

        let cli: Cli = Cli::try_parse_from(args).unwrap();
        cli.run().await.unwrap();

        // Assert
        let validator_rpc_client = validator.get_async_rpc_client();
        let accounts = validator_rpc_client
            .get_program_accounts(&program_id)
            .await
            .unwrap();

        let account = accounts.first().unwrap().clone().1;

        assert_eq!(account.owner, gmp_gateway::id());

        // Expected values from the tests/auth_weighted.toml file
        let sig1_address =
            Address::try_from("092c3da15c17a1e3eb01ed279684cc197a9938bde2dc1e59835a61afa6fb17ad64")
                .unwrap();
        let sig1_weight = U256::from(1u8);
        let sig2_address =
            Address::try_from("508efe1eb50545edd0f762ba61290c579d513a38239bebaa97379628cefe82e62d")
                .unwrap();
        let sig2_weight = U256::from(2u8);

        let signers_and_weights = [(sig1_address, sig1_weight), (sig2_address, sig2_weight)];
        let auth_weighted = AxelarAuthWeighted::new(
            signers_and_weights.iter().map(|(a, w)| (a, *w)),
            signers_and_weights
                .iter()
                .fold(U256::ZERO, |a, b| a.checked_add(b.1).unwrap()),
        );
        let (_, bump) = GatewayConfig::pda();
        let gateway_config = GatewayConfig::new(bump, auth_weighted);
        let deserialized_gateway_config: GatewayConfig = from_slice(&account.data).unwrap();
        assert_eq!(deserialized_gateway_config, gateway_config);
    }
}
