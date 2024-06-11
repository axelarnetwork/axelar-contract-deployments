use std::fmt::Display;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use axelar_message_primitives::command::U256;
use axelar_message_primitives::Address;
use gmp_gateway::axelar_auth_weighted::AxelarAuthWeighted;
use gmp_gateway::state::GatewayConfig;
use serde::Deserialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use tracing::info;
use url::Url;
use xshell::{cmd, Shell};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub(crate) enum SolanaContract {
    GmpGateway,
}

impl SolanaContract {
    /// Provides the predictable output artifact that will be
    /// generated when each contract it's built. This is a helper
    /// method that is normally join'`ed()` with other base directories.
    pub(crate) fn file(self) -> PathBuf {
        match self {
            SolanaContract::GmpGateway => PathBuf::from("gmp_gateway.so"),
        }
    }
    /// Provides the local folder name at "solana/programs" each
    /// contract belongs to.
    /// This is a helper method that is normally when it's needed to
    /// i.e "cd" into the contract folder for building it with `cargo-sbf`.
    pub(crate) fn dir(self) -> PathBuf {
        match self {
            SolanaContract::GmpGateway => PathBuf::from("gateway"),
        }
    }
}

impl Display for SolanaContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolanaContract::GmpGateway => write!(f, "gmp-gateway"),
        }
    }
}

pub(crate) fn deploy(
    contract: SolanaContract,
    program_id: &Path,
    keypair_path: &Option<PathBuf>,
    url: &Option<Url>,
    ws_url: &Option<Url>,
) -> anyhow::Result<()> {
    crate::cli::cmd::path::ensure_optional_path_exists(keypair_path.as_ref(), "keypair")?;

    info!("Starting compiling {}", contract);
    build_contract(contract)?;
    info!("Compiled {}", contract);

    info!("Starting deploying {}", contract);
    let pub_key = deploy_contract(contract, program_id, keypair_path, url, ws_url)?;
    info!("Deployed {contract} at {pub_key:?}");
    Ok(())
}

pub(crate) async fn init_gmp_gateway(
    auth_weighted: &PathBuf,
    rpc_url: &Option<Url>,
    payer_kp_path: &Option<PathBuf>,
) -> anyhow::Result<()> {
    let payer_kp = defaults::payer_kp_with_fallback_in_sol_cli_config(payer_kp_path)?;

    let (gateway_config_pda, bump) = GatewayConfig::pda();

    // Read toml file data
    let gateway_config_file_content = read_to_string(auth_weighted)?;
    let gateway_config_data = toml::from_str::<GatewayConfigData>(&gateway_config_file_content)?;

    let auth_weighted = AxelarAuthWeighted::new(
        gateway_config_data
            .signers
            .iter()
            .map(|signer| (&signer.address, U256::from(signer.weight as u128))),
        gateway_config_data.calc_signer_thershold(),
    );

    let gateway_config = GatewayConfig::new(bump, auth_weighted, gateway_config_data.operator);

    let ix = gmp_gateway::instructions::initialize_config(
        payer_kp.pubkey(),
        gateway_config,
        gateway_config_pda,
    )?;

    let rpc_client =
        RpcClient::new(defaults::rpc_url_with_fallback_in_sol_cli_config(rpc_url)?.to_string());
    let recent_hash = rpc_client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_kp.pubkey()),
        &[&payer_kp],
        recent_hash,
    );

    let _signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)
        .await?;

    Ok(())
}

/// An intermediate struct for parsing
/// values from a TOML file.
#[derive(Deserialize, Debug)]
struct GatewayConfigData {
    signers: Vec<GatewaySigner>,
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    operator: Pubkey,
}

impl GatewayConfigData {
    fn calc_signer_thershold(&self) -> U256 {
        self.signers.iter().fold(U256::ZERO, |a, b| {
            a.checked_add(U256::from(b.weight as u128)).unwrap()
        })
    }
}

#[derive(Deserialize, Debug)]
struct GatewaySigner {
    #[serde(deserialize_with = "serde_utils::deserialize_address")]
    address: Address,
    weight: u64,
}

mod serde_utils {
    use serde::Deserializer;

    use super::{Address, Deserialize, FromStr, Pubkey};

    pub(crate) fn deserialize_address<'de, D>(deserializer: D) -> Result<Address, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        Address::try_from(raw_string.as_str()).map_err(serde::de::Error::custom)
    }

    pub(crate) fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        Pubkey::from_str(&raw_string).map_err(serde::de::Error::custom)
    }
}

pub(crate) fn build_contract(contract: SolanaContract) -> anyhow::Result<PathBuf> {
    let contract_dir = path::contracts_dir().join(contract.dir());
    let sh = Shell::new()?;
    sh.change_dir(contract_dir);
    cmd!(sh, "cargo build-bpf").run()?;
    Ok(path::contracts_artifact_dir().join(contract.file()))
}

fn deploy_contract(
    contract: SolanaContract,
    program_id: &Path,
    keypair_path: &Option<PathBuf>,
    url: &Option<Url>,
    ws_url: &Option<Url>,
) -> anyhow::Result<Pubkey> {
    let contract_compiled_binary = path::contracts_artifact_dir().join(contract.file());
    let sh = Shell::new()?;
    let deploy_cmd_args = calculate_deploy_cmd_args(
        program_id,
        keypair_path.as_ref(),
        url.as_ref(),
        ws_url.as_ref(),
        &contract_compiled_binary,
    );

    let program_id_output = cmd!(sh, "solana program deploy {deploy_cmd_args...}").read()?;

    parse_program_id(&program_id_output)
}

fn parse_program_id(output: &str) -> anyhow::Result<Pubkey> {
    let parts: Vec<&str> = output.split(':').collect();
    let id_part: &&str = parts.get(1).ok_or(anyhow::anyhow!(
        "Cannot parse programId from parts. Expected second index not found."
    ))?;
    Ok(Pubkey::from_str(id_part.trim())?)
}

fn calculate_deploy_cmd_args(
    program_id: &Path,
    keypair_path: Option<&PathBuf>,
    url: Option<&Url>,
    ws_url: Option<&Url>,
    contract_compiled_binary_path: &Path,
) -> Vec<String> {
    let mut cmd = vec![
        "--program-id".to_string(),
        program_id.to_string_lossy().to_string(),
    ];

    if let Some(kp) = keypair_path {
        cmd.push("-k".to_string());
        cmd.push(kp.to_string_lossy().to_string());
    }

    if let Some(url) = url {
        cmd.push("-u".to_string());
        cmd.push(url.to_string());
    }

    if let Some(ws_url) = ws_url {
        cmd.push("--ws".to_string());
        cmd.push(ws_url.to_string());
    }
    let compiled_bin_path = contract_compiled_binary_path.to_string_lossy();
    cmd.push(compiled_bin_path.to_string());
    cmd
}

pub(crate) mod path {
    use std::path::PathBuf;

    use crate::cli::cmd::path::workspace_root_dir;

    pub(crate) fn contracts_dir() -> PathBuf {
        workspace_root_dir().join("programs")
    }

    pub(crate) fn contracts_artifact_dir() -> PathBuf {
        workspace_root_dir().join("target").join("deploy")
    }
}

mod defaults {

    use std::path::PathBuf;
    use std::str::FromStr;

    use solana_cli_config::Config;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::EncodableKey;
    use url::Url;

    /// If provided, it parses the Keypair from the provided
    /// path. If not provided, it calculates and uses default Solana CLI
    /// keypair path. Finally, it tries to read the file.
    pub(crate) fn payer_kp_with_fallback_in_sol_cli_config(
        payer_kp_path: &Option<PathBuf>,
    ) -> anyhow::Result<Keypair> {
        let calculated_payer_kp_path = match payer_kp_path {
            Some(kp_path) => kp_path.clone(),
            None => PathBuf::from(Config::default().keypair_path),
        };
        crate::cli::cmd::path::ensure_path_exists(&calculated_payer_kp_path, "payer keypair")?;
        Keypair::read_from_file(&calculated_payer_kp_path)
            .map_err(|_| anyhow::Error::msg("Could not read payer key pair"))
    }

    /// If provided, it parses the provided RPC URL. If not provided,
    /// it calculates and uses default Solana CLI
    /// rpc URL.
    pub(crate) fn rpc_url_with_fallback_in_sol_cli_config(
        rpc_url: &Option<Url>,
    ) -> anyhow::Result<Url> {
        let calculated_rpc_url = match rpc_url {
            Some(kp_path) => kp_path.clone(),
            None => {
                #[allow(deprecated)]
                // We are not explicitly supporting windows, plus home_dir() is what solana is using
                // under the hood.
                let mut sol_config_path =
                    std::env::home_dir().ok_or(anyhow::anyhow!("Home dir not found !"))?;
                sol_config_path.extend([".config", "solana", "cli", "config.yml"]);

                let sol_cli_config = Config::load(
                    sol_config_path
                        .to_str()
                        .ok_or(anyhow::anyhow!("Config path not valid unicode !"))?,
                )?;
                Url::from_str(&sol_cli_config.json_rpc_url)?
            }
        };

        Ok(calculated_rpc_url)
    }
}

#[cfg(test)]
mod tests {

    use anyhow::Ok;

    use super::*;

    #[test]
    fn parse_program_id_from_deploy_output() {
        let expected_output =
            Pubkey::from_str("4gG8FWzYihgixVfEdgGkMSdRTN9q8cGyDbkVwR72ir1g").unwrap();
        let cases = vec![
            (
                "ProgramId: 4gG8FWzYihgixVfEdgGkMSdRTN9q8cGyDbkVwR72ir1g",
                expected_output,
            ),
            (
                "ProgramId:4gG8FWzYihgixVfEdgGkMSdRTN9q8cGyDbkVwR72ir1g",
                expected_output,
            ),
            (
                "ProgramId: 4gG8FWzYihgixVfEdgGkMSdRTN9q8cGyDbkVwR72ir1g    ",
                expected_output,
            ),
            (
                "PROGRAMID: 4gG8FWzYihgixVfEdgGkMSdRTN9q8cGyDbkVwR72ir1g",
                expected_output,
            ),
        ];

        cases
            .into_iter()
            .try_for_each(|(input, expected)| {
                let pubkey = parse_program_id(input)?;
                assert_eq!(
                    pubkey, expected,
                    "We expected input {input} to be parsed to {expected}"
                );
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn calc_deploy_cmd_when_no_params_it_takes_default_solana_cli_config() {
        let kp = None;
        let url = None;
        let ws_url = None;
        let program_id = PathBuf::from_str("~/path/program-id-keypair.json").unwrap();

        let result = calculate_deploy_cmd_args(
            &program_id,
            kp,
            url,
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
            "--program-id",
            program_id.to_string_lossy().to_string().as_str(),
            "/contracts/contract.so",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn calc_deploy_cmd_when_only_key_pair() {
        let kp = Some(PathBuf::from_str("/path/keypair.txt").unwrap());
        let url = None;
        let ws_url = None;
        let program_id = PathBuf::from_str("~/path/program-id-keypair.json").unwrap();

        let result = calculate_deploy_cmd_args(
            &program_id,
            kp.as_ref(),
            url,
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
            "--program-id",
            program_id.to_string_lossy().to_string().as_str(),
            "-k",
            "/path/keypair.txt",
            "/contracts/contract.so",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn calc_deploy_cmd_when_only_url() {
        let kp = None;
        let url = Some(Url::from_str("http://127.0.0.1:3333/").unwrap());
        let ws_url = None;
        let program_id = PathBuf::from_str("~/path/program-id-keypair.json").unwrap();

        let result = calculate_deploy_cmd_args(
            &program_id,
            kp,
            url.as_ref(),
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
            "--program-id",
            program_id.to_string_lossy().to_string().as_str(),
            "-u",
            "http://127.0.0.1:3333/",
            "/contracts/contract.so",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn calc_deploy_cmd_when_only_ws_url() {
        let kp = None;
        let url = None;
        let ws_url = Some(Url::from_str("http://127.0.0.1:3333/").unwrap());
        let program_id = PathBuf::from_str("~/path/program-id-keypair.json").unwrap();

        let result = calculate_deploy_cmd_args(
            &program_id,
            kp,
            url,
            ws_url.as_ref(),
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
            "--program-id",
            program_id.to_string_lossy().to_string().as_str(),
            "--ws",
            "http://127.0.0.1:3333/",
            "/contracts/contract.so",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn calc_deploy_cmd_when_full_params_provided() {
        let kp = Some(PathBuf::from_str("/path/keypair.txt").unwrap());
        let url = Some(Url::from_str("http://127.0.0.1:2222").unwrap());
        let ws_url = Some(Url::from_str("http://127.0.0.1:3333").unwrap());
        let program_id = PathBuf::from_str("~/path/program-id-keypair.json").unwrap();

        let result = calculate_deploy_cmd_args(
            &program_id,
            kp.as_ref(),
            url.as_ref(),
            ws_url.as_ref(),
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
            "--program-id",
            program_id.to_string_lossy().to_string().as_str(),
            "-k",
            "/path/keypair.txt",
            "-u",
            "http://127.0.0.1:2222/",
            "--ws",
            "http://127.0.0.1:3333/",
            "/contracts/contract.so",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        assert_eq!(expected, result);
    }

    #[test]
    fn calc_threshold_from_works() {
        let config = GatewayConfigData {
            signers: vec![
                GatewaySigner {
                    address: Address::try_from(
                        "07453457a565724079d7dfab633d026d49cac3f6d69bce20bc79adedfccdf69ab2",
                    )
                    .unwrap(),
                    weight: 1,
                },
                GatewaySigner {
                    address: Address::try_from(
                        "6b322380108ca6c6313667657aab424ad0ea014cf3fb107bb124e8822bc9d0befb",
                    )
                    .unwrap(),
                    weight: 2,
                },
            ],
            operator: Pubkey::new_unique(),
        };

        assert_eq!(U256::from(3u128), config.calc_signer_thershold());
    }
}
