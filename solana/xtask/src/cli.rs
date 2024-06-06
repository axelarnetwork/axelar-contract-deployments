use std::path::PathBuf;

use clap::{Parser, Subcommand};
use cmd::solana::SolanaContract;
use report::Report;
use url::Url;

mod cmd;
pub(crate) mod report;

#[cfg(test)]
mod test_helpers;

/// Xtask is the Axelar Solana workspace CLI that helps
/// both actors, humans and CI to achieve mundane tasks
/// like building, deploying and initializing Solana
/// programs.
#[derive(Parser)]
#[command(version, about, long_about = None)]
pub(crate) enum Cli {
    Solana {
        #[command(subcommand)]
        solana_command: Solana,
    },
}

#[derive(Subcommand)]
pub(crate) enum Solana {
    /// Build's a contract that is listed in the programs
    /// workspace directory.
    Build {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: SolanaContract,
    },
    /// Deploys the given contract name
    Deploy {
        /// It accepts the name of the contract folder as argument.
        #[arg(value_enum)]
        contract: SolanaContract,
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
        contract: SolanaInitSubcommand,
    },
}

/// Initialize contracts by providing their specific init parameters.
#[derive(Subcommand)]
pub(crate) enum SolanaInitSubcommand {
    /// Initialize an already deployed gateway contract.
    GmpGateway {
        /// A path that points to a toml file that contains the signers and
        /// their respective weights data. See `tests/auth_weighted.toml` file
        /// for an example.
        #[arg(short, long)]
        auth_weighted_file: PathBuf,
        /// The RPC URL of the target validator.
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        rpc_url: Option<Url>,
        /// The payer keypair file. This is a file containing the byte slice
        /// serialization of a `solana_sdk::signer::keypair::Keypair` .
        /// If not provided, this will fallback in solana CLI current
        /// configuration.
        #[arg(short, long)]
        payer_kp_path: Option<PathBuf>,
    },
}

impl Cli {
    pub(crate) async fn run(&self) -> anyhow::Result<Report> {
        match self {
            Cli::Solana { solana_command } => match solana_command {
                Solana::Build { contract } => {
                    cmd::solana::build_contract(*contract).map(Report::Build)
                }
                Solana::Deploy {
                    contract,
                    keypair_path,
                    url,
                    ws_url,
                } => cmd::solana::deploy(*contract, keypair_path, url, ws_url),
                Solana::Init { contract } => match &contract {
                    SolanaInitSubcommand::GmpGateway {
                        auth_weighted_file,
                        rpc_url,
                        payer_kp_path,
                    } => {
                        cmd::solana::init_gmp_gateway(auth_weighted_file, rpc_url, payer_kp_path)
                            .await
                    }
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {

    use std::io::Write;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use axelar_message_primitives::command::U256;
    use axelar_message_primitives::Address;
    use borsh::from_slice;
    use gmp_gateway::axelar_auth_weighted::AxelarAuthWeighted;
    use gmp_gateway::state::GatewayConfig;
    use serial_test::serial;
    use solana_test_validator::{TestValidatorGenesis, UpgradeableProgramInfo};
    use tempfile::NamedTempFile;
    use tests::test_helpers::build_gateway_contract;

    use super::*;

    #[tokio::test]
    #[serial]
    async fn build_actually_works() {
        // setup
        let args = vec!["xtask", "solana", "build", "gmp-gateway"];
        let cli: Cli = Cli::try_parse_from(args).unwrap();

        // action
        let result = cli.run().await.unwrap();

        // assert
        let contract_path = match result {
            Report::Build(report) => report,
            _ => panic!("result not expected."),
        };
        assert!(contract_path.exists());
    }

    #[tokio::test]
    #[serial]
    async fn deploy_actually_works() {
        // setup
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

        // action
        let args = vec![
            "xtask",
            "solana",
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
        let contract_id = match result {
            Report::Deploy(report) => report,
            _ => panic!("result not expected."),
        };

        // assert
        let validator_rpc_client = validator.get_async_rpc_client();
        let account_info = validator_rpc_client
            .get_account(&contract_id)
            .await
            .unwrap();
        assert!(account_info.executable);
    }

    #[tokio::test]
    #[serial]
    async fn initialize_gateway_contract_works() {
        // Setup
        solana_logger::setup_with_default("solana_program_runtime=warn");
        build_gateway_contract();
        // Bring up the validator + the target contract to initialise.
        let mut seed_validator = TestValidatorGenesis::default();
        let program_id = gmp_gateway::id();
        seed_validator.add_upgradeable_programs_with_path(&[UpgradeableProgramInfo {
            program_id,
            loader: solana_sdk::bpf_loader_upgradeable::id(),
            upgrade_authority: program_id,
            program_path: cmd::solana::path::contracts_artifact_dir()
                .join(SolanaContract::GmpGateway.file()),
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
            "solana",
            "init",
            "gmp-gateway",
            "--rpc-url",
            &rpc_url,
            "--payer-kp-path",
            &payer_kp, // We use the already funded keypair.
            "--auth-weighted-file",
            "tests/gateway_init_config.toml",
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
        let hardcoded_operator =
            solana_sdk::pubkey::Pubkey::from_str("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL")
                .unwrap();

        let signers_and_weights = [(sig1_address, sig1_weight), (sig2_address, sig2_weight)];
        let auth_weighted = AxelarAuthWeighted::new(
            signers_and_weights.iter().map(|(a, w)| (a, *w)),
            signers_and_weights
                .iter()
                .fold(U256::ZERO, |a, b| a.checked_add(b.1).unwrap()),
        );
        let (_, bump) = GatewayConfig::pda();

        let gateway_config = GatewayConfig::new(bump, auth_weighted, hardcoded_operator);
        let deserialized_gateway_config = from_slice::<GatewayConfig>(&account.data).unwrap();
        assert_eq!(deserialized_gateway_config, gateway_config);
    }
}
