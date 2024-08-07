use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use axelar_rkyv_encoding::types::{PublicKey, VerifierSet, U128};
use gmp_gateway::axelar_auth_weighted::AxelarAuthWeighted;
use gmp_gateway::state::GatewayConfig;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer;
use solana_sdk::transaction::Transaction;
use tracing::info;
use url::Url;
use xshell::{cmd, Shell};

use super::cosmwasm::cosmos_client::signer::SigningClient;
use super::testnet::solana_domain_separator;
use super::testnet::solana_interactions::send_solana_tx;
use crate::cli::cmd::testnet::multisig_prover_api;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub(crate) enum SolanaContract {
    GmpGateway,
    AxelarSolanaMemo,
}

impl SolanaContract {
    /// Provides the predictable output artifact that will be
    /// generated when each contract it's built. This is a helper
    /// method that is normally join'`ed()` with other base directories.
    pub(crate) fn file(self) -> PathBuf {
        match self {
            SolanaContract::GmpGateway => PathBuf::from("gmp_gateway.so"),
            SolanaContract::AxelarSolanaMemo => PathBuf::from("axelar_solana_memo_program.so"),
        }
    }
}

impl Display for SolanaContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolanaContract::GmpGateway => write!(f, "gmp-gateway"),
            SolanaContract::AxelarSolanaMemo => write!(f, "axelar-solana-memo-program"),
        }
    }
}

pub(crate) fn deploy(
    contract: SolanaContract,
    program_id: &Path,
    keypair_path: Option<&PathBuf>,
    url: Option<&Url>,
    ws_url: Option<&Url>,
) -> eyre::Result<()> {
    crate::cli::cmd::path::ensure_optional_path_exists(keypair_path, "keypair")?;

    info!("Starting compiling {}", contract);
    build_contracts()?;
    info!("Compiled {}", contract);

    info!("Starting deploying {}", contract);
    let pub_key = deploy_contract(contract, program_id, keypair_path, url, ws_url)?;
    info!("Deployed {contract} at {pub_key:?}");
    Ok(())
}

pub(crate) async fn init_gmp_gateway(
    rpc_url: Option<&Url>,
    payer_kp_path: Option<&PathBuf>,
    destination_multisig_prover: &str,
    cosmwasm_signer: SigningClient,
) -> eyre::Result<()> {
    let payer_kp = defaults::payer_kp_with_fallback_in_sol_cli_config(payer_kp_path)?;

    let (gateway_config_pda, bump) = GatewayConfig::pda();

    // Query the cosmwasm multisig prover to get the latest verifier set
    let destination_multisig_prover = cosmrs::AccountId::from_str(destination_multisig_prover)?;
    let res = cosmwasm_signer
        .query::<multisig_prover_api::VerifierSetResponse>(
            destination_multisig_prover.clone(),
            serde_json::to_vec(&multisig_prover_api::QueryMsg::CurrentVerifierSet {})?,
        )
        .await?;

    let mut signers = BTreeMap::new();
    for signer in res.verifier_set.signers.values() {
        let pubkey = PublicKey::new_ecdsa(signer.pub_key.as_ref().try_into()?);
        let weight = U128::from(signer.weight.u128());
        signers.insert(pubkey, weight);
    }
    let verifier_set = VerifierSet::new(
        res.verifier_set.created_at,
        signers,
        U128::from(res.verifier_set.threshold.u128()),
    );
    tracing::info!(
        returned = ?res.verifier_set,
        "returned verifier set"
    );
    tracing::info!(
        reconstructed = ?verifier_set,
        "reconstructed verifier set"
    );
    let auth_weighted = AxelarAuthWeighted::new(verifier_set);
    tracing::info!(?auth_weighted, "initting auth weighted");

    let gateway_config = GatewayConfig::new(
        bump,
        auth_weighted,
        payer_kp.pubkey(),
        solana_domain_separator(),
    );

    let ix = gmp_gateway::instructions::initialize_config(
        payer_kp.pubkey(),
        gateway_config,
        gateway_config_pda,
    )?;

    let rpc_client =
        RpcClient::new(defaults::rpc_url_with_fallback_in_sol_cli_config(rpc_url)?.to_string());
    let recent_hash = rpc_client.get_latest_blockhash()?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer_kp.pubkey()),
        &[&payer_kp],
        recent_hash,
    );

    let _signature = rpc_client.send_and_confirm_transaction_with_spinner(&tx)?;

    Ok(())
}

pub(crate) fn init_memo_program(
    // todo change all instances of &Opion<X> to Option<&X>
    rpc_url: Option<&Url>,
    payer_kp_path: Option<&PathBuf>,
) -> eyre::Result<()> {
    let payer_kp = defaults::payer_kp_with_fallback_in_sol_cli_config(payer_kp_path)?;
    let rpc_client =
        RpcClient::new(defaults::rpc_url_with_fallback_in_sol_cli_config(rpc_url)?.to_string());

    let gateway_root_pda = gmp_gateway::get_gateway_root_config_pda().0;
    let counter = axelar_solana_memo_program::get_counter_pda(&gateway_root_pda);
    let ix = axelar_solana_memo_program::instruction::initialize(
        &payer_kp.pubkey(),
        &gateway_root_pda,
        &counter,
    )?;
    send_solana_tx(&rpc_client, &[ix], &payer_kp);
    Ok(())
}

pub(crate) fn build_contracts() -> eyre::Result<()> {
    let sh = Shell::new()?;
    cmd!(sh, "cargo build-sbf").run()?;
    Ok(())
}

fn deploy_contract(
    contract: SolanaContract,
    program_id: &Path,
    keypair_path: Option<&PathBuf>,
    url: Option<&Url>,
    ws_url: Option<&Url>,
) -> eyre::Result<Pubkey> {
    let contract_compiled_binary = path::contracts_artifact_dir().join(contract.file());
    let sh = Shell::new()?;
    let deploy_cmd_args = calculate_deploy_cmd_args(
        program_id,
        keypair_path,
        url,
        ws_url,
        &contract_compiled_binary,
    );

    let program_id_output = cmd!(sh, "solana program deploy {deploy_cmd_args...}").read()?;

    parse_program_id(&program_id_output)
}

fn parse_program_id(output: &str) -> eyre::Result<Pubkey> {
    let parts: Vec<&str> = output.split(':').collect();
    let id_part: &&str = parts.get(1).ok_or(eyre::eyre!(
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

    pub(crate) fn contracts_artifact_dir() -> PathBuf {
        workspace_root_dir().join("target").join("deploy")
    }
}

pub(crate) mod defaults {

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
        payer_kp_path: Option<&PathBuf>,
    ) -> eyre::Result<Keypair> {
        let calculated_payer_kp_path = match payer_kp_path {
            Some(kp_path) => kp_path.clone(),
            None => PathBuf::from(Config::default().keypair_path),
        };
        crate::cli::cmd::path::ensure_path_exists(&calculated_payer_kp_path, "payer keypair")?;
        Keypair::read_from_file(&calculated_payer_kp_path)
            .map_err(|_| eyre::Error::msg("Could not read payer key pair"))
    }

    /// If provided, it parses the provided RPC URL. If not provided,
    /// it calculates and uses default Solana CLI
    /// rpc URL.
    pub(crate) fn rpc_url_with_fallback_in_sol_cli_config(
        rpc_url: Option<&Url>,
    ) -> eyre::Result<Url> {
        let calculated_rpc_url = if let Some(kp_path) = rpc_url {
            kp_path.clone()
        } else {
            #[allow(deprecated)]
            // We are not explicitly supporting windows, plus home_dir() is what solana is using
            // under the hood.
            let mut sol_config_path =
                std::env::home_dir().ok_or(eyre::eyre!("Home dir not found !"))?;
            sol_config_path.extend([".config", "solana", "cli", "config.yml"]);

            let sol_cli_config = Config::load(
                sol_config_path
                    .to_str()
                    .ok_or(eyre::eyre!("Config path not valid unicode !"))?,
            )?;
            Url::from_str(&sol_cli_config.json_rpc_url)?
        };

        Ok(calculated_rpc_url)
    }
}

#[cfg(test)]
mod tests {

    use eyre::Ok;

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
}
