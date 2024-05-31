use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use axelar_message_primitives::command::U256;
use axelar_message_primitives::Address;
use defaults::{payer_kp_with_fallback_in_sol_cli_config, rpc_url_with_fallback_in_sol_cli_config};
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

use super::path::{contracts_artifact_dir, contracts_dir};
use super::report::Report;
use super::Contract;
use crate::cli::path::ensure_optional_path_exists;

mod defaults;

pub async fn deploy(
    contract: &Contract,
    keypair_path: &Option<PathBuf>,
    url: &Option<Url>,
    ws_url: &Option<Url>,
) -> anyhow::Result<Report> {
    ensure_optional_path_exists(keypair_path.as_ref(), "keypair")?;

    info!("Starting compiling {}", contract);
    build_contract(contract).await?;
    info!("Compiled {}", contract);

    info!("Starting deploying {}", contract);
    let pub_key = deploy_contract(contract, keypair_path, url, ws_url).await?;
    info!("Deployed {}", contract);
    Ok(Report::Deploy(pub_key))
}

pub async fn init_gmp_gateway(
    auth_weighted: &PathBuf,
    rpc_url: &Option<Url>,
    payer_kp_path: &Option<PathBuf>,
) -> anyhow::Result<Report> {
    let payer_kp = payer_kp_with_fallback_in_sol_cli_config(payer_kp_path)?;

    let (gateway_config_pda, bump) = GatewayConfig::pda();

    // Read toml file data
    let auth_weighted_file_content = read_to_string(auth_weighted)?;
    let auth_weighted_data: AuthWeightedData = toml::from_str(&auth_weighted_file_content)?;
    let mut signers_and_weights: Vec<(Address, U256)> = Vec::new();

    for (addr, weight) in auth_weighted_data
        .signers
        .iter()
        .zip(auth_weighted_data.weights)
    {
        signers_and_weights.push((
            Address::try_from(addr.as_str())?,
            U256::from(weight as u128),
        ))
    }

    let auth_weighted = AxelarAuthWeighted::new(
        signers_and_weights
            .iter()
            .map(|(addr, weight)| (addr, *weight)),
        calc_threshold_from(&signers_and_weights),
    );

    let gateway_config = GatewayConfig::new(bump, auth_weighted);

    let ix = gmp_gateway::instructions::initialize_config(
        payer_kp.pubkey(),
        gateway_config,
        gateway_config_pda,
    )?;

    let rpc_client = RpcClient::new(rpc_url_with_fallback_in_sol_cli_config(rpc_url)?.to_string());
    let recent_hash = rpc_client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_kp.pubkey()),
        &[&payer_kp],
        recent_hash,
    );

    let signature = rpc_client
        .send_and_confirm_transaction_with_spinner(&tx)
        .await?;

    Ok(Report::Init(signature.to_string()))
}

/// An intermediate struct for parsing
/// values from a TOML file.
#[derive(Deserialize)]
struct AuthWeightedData {
    signers: Vec<String>,
    weights: Vec<u64>,
}

fn calc_threshold_from(signers_and_weights: &[(Address, U256)]) -> U256 {
    signers_and_weights
        .iter()
        .fold(U256::ZERO, |a, b| a.checked_add(b.1).unwrap())
}

pub async fn build_contract(contract: &Contract) -> anyhow::Result<PathBuf> {
    let contract_dir = contracts_dir()?.join(contract.dir());
    let sh = Shell::new()?;
    sh.change_dir(contract_dir);
    cmd!(sh, "cargo build-bpf").run()?;
    Ok(contracts_artifact_dir()?.join(contract.file()))
}

async fn deploy_contract(
    contract: &Contract,
    keypair_path: &Option<PathBuf>,
    url: &Option<Url>,
    ws_url: &Option<Url>,
) -> anyhow::Result<Pubkey> {
    let contract_compiled_binary = contracts_artifact_dir()?.join(contract.file());
    let sh = Shell::new()?;
    let deploy_cmd_args = calculate_deploy_cmd_args(
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
    keypair_path: Option<&PathBuf>,
    url: Option<&Url>,
    ws_url: Option<&Url>,
    contract_compiled_binary_path: &Path,
) -> Vec<String> {
    let mut cmd = Vec::new();

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
                    "We expected input {} to be parsed to {}",
                    input, expected
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

        let result = calculate_deploy_cmd_args(
            kp,
            url,
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec!["/contracts/contract.so"]
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

        let result = calculate_deploy_cmd_args(
            kp.as_ref(),
            url,
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec!["-k", "/path/keypair.txt", "/contracts/contract.so"]
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

        let result = calculate_deploy_cmd_args(
            kp,
            url.as_ref(),
            ws_url,
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec!["-u", "http://127.0.0.1:3333/", "/contracts/contract.so"]
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

        let result = calculate_deploy_cmd_args(
            kp,
            url,
            ws_url.as_ref(),
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> =
            vec!["--ws", "http://127.0.0.1:3333/", "/contracts/contract.so"]
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

        let result = calculate_deploy_cmd_args(
            kp.as_ref(),
            url.as_ref(),
            ws_url.as_ref(),
            &PathBuf::from_str("/contracts/contract.so").unwrap(),
        );

        let expected: Vec<String> = vec![
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
        let signers_and_weights = vec![
            (
                Address::try_from(
                    "07453457a565724079d7dfab633d026d49cac3f6d69bce20bc79adedfccdf69ab2",
                )
                .unwrap(),
                U256::from(1u128),
            ),
            (
                Address::try_from(
                    "6b322380108ca6c6313667657aab424ad0ea014cf3fb107bb124e8822bc9d0befb",
                )
                .unwrap(),
                U256::from(2u128),
            ),
        ];

        assert_eq!(U256::from(3u128), calc_threshold_from(&signers_and_weights))
    }
}
