use std::io::Write;
use std::str::FromStr;

use cosmrs::cosmwasm::{MsgExecuteContract, MsgInstantiateContract, MsgStoreCode};
use cosmrs::tx::Msg;
use cosmrs::Denom;
use eyre::OptionExt;
use k256::elliptic_curve::rand_core::OsRng;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_sdk::keccak::hashv;
use xshell::Shell;

mod build;
pub(crate) mod cosmos_client;

use build::{build_contracts, download_wasm_opt, setup_toolchain, unpack_tar_gz};

use self::path::{binaryen_tar_file, binaryen_unpacked, wasm_opt_binary};
use super::deployments::{AxelarConfiguration, SolanaDeploymentRoot};
use crate::cli::cmd::cosmwasm::cosmos_client::gas::Gas;
use crate::cli::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use crate::cli::cmd::deployments::{
    AxelarGatewayDeployment, MultisigProverDeployment, VotingVerifierDeployment,
};
use crate::cli::cmd::testnet::multisig_prover_api;

struct WasmContracts {
    wasm_artifact_name: &'static str,
    contract_project_folder: &'static str,
}

const CONTRACTS: [WasmContracts; 3] = [
    WasmContracts {
        wasm_artifact_name: "voting_verifier",
        contract_project_folder: "voting-verifier",
    },
    WasmContracts {
        wasm_artifact_name: "gateway",
        contract_project_folder: "gateway",
    },
    WasmContracts {
        wasm_artifact_name: "multisig_prover",
        contract_project_folder: "multisig-prover",
    },
];

pub(crate) async fn build() -> eyre::Result<()> {
    let sh = Shell::new()?;

    // install `wasm-opt` if it doesn't already exist
    if !wasm_opt_binary().exists() {
        tracing::info!("wasm opt does not exist - will download and unpack");
        let binaryen_archive = binaryen_tar_file();
        download_wasm_opt(binaryen_archive.as_path()).await?;
        unpack_tar_gz(binaryen_archive.as_path(), binaryen_unpacked().as_path())?;
    }

    // set up `axelar-amplifier`-specific toolchain
    let _toolchain = setup_toolchain(&sh)?;
    build_contracts(&sh, &wasm_opt_binary(), &CONTRACTS).await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
pub(crate) async fn deploy(
    client: &SigningClient,
    config: &mut AxelarConfiguration,
) -> eyre::Result<()> {
    // deploy each contract - do not instantiate
    config
        .voting_verifier_code_id
        .replace(deploy_contract(&CONTRACTS[0], client, config).await?);
    config
        .gateway_code_id
        .replace(deploy_contract(&CONTRACTS[1], client, config).await?);
    config
        .multisig_prover_code_id
        .replace(deploy_contract(&CONTRACTS[2], client, config).await?);
    Ok(())
}

async fn deploy_contract(
    contract: &WasmContracts,
    client: &SigningClient,
    config: &AxelarConfiguration,
) -> Result<u64, eyre::Error> {
    tracing::info!(contract = ?contract.wasm_artifact_name, "about to deploy contract");
    let wasm_byte_code = read_wasm_for_deployment(contract.wasm_artifact_name)?;
    let msg_store_code = MsgStoreCode {
        sender: client.signer_account_id()?,
        wasm_byte_code,
        instantiate_permission: None,
    }
    .to_any()?;
    let response = client
        .sign_and_broadcast(vec![msg_store_code], &default_gas(config)?)
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");
    let code_id = response.extract("store_code", "code_id")?;
    tracing::info!(code_id, contract = ?contract.wasm_artifact_name, "code stored");
    let code_id = code_id.parse()?;
    Ok(code_id)
}

#[tracing::instrument(skip_all)]
pub(crate) async fn init_solana_voting_verifier(
    client: &SigningClient,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<String> {
    use voting_verifier::msg::InstantiateMsg;
    tracing::info!("init voting verifier");

    let code_id = solana_deployment_root
        .axelar_configuration
        .voting_verifier_code_id
        .ok_or_eyre("voting verifier code id not present. Was it deployed?")?;
    let instantiate_msg = InstantiateMsg {
        address_format: axelar_wasm_std::address::AddressFormat::Base58Solana,
        service_registry_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .service_registry
            .address
            .to_string()
            .try_into()?,
        service_name: solana_deployment_root
            .axelar_configuration
            .service_name
            .to_string()
            .try_into()?,
        source_gateway_address: solana_deployment_root
            .solana_configuration
            .gateway_program_id
            .clone()
            .try_into()?,
        voting_threshold: majority_threshold(&solana_deployment_root.axelar_configuration),
        block_expiry: solana_deployment_root
            .axelar_configuration
            .voting_verifier_block_expiry
            .try_into()?,
        confirmation_height: solana_deployment_root
            .axelar_configuration
            .voting_verifier_confirmation_height,
        source_chain: solana_deployment_root
            .solana_configuration
            .chain_name_on_axelar_chain
            .to_string()
            .try_into()?,
        rewards_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .rewards
            .address
            .to_string()
            .try_into()?,
        governance_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .service_registry
            .governance_account
            .to_string()
            .try_into()?,
        msg_id_format: solana_deployment_root
            .axelar_configuration
            .voting_verifier_msg_id_format
            .clone(),
    };
    tracing::info!(?instantiate_msg, "instantiate msg");
    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("voting-verifier".to_string()),
        msg: serde_json::to_vec(&instantiate_msg)?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(
            vec![instantiate.into_any()?],
            &default_gas(&solana_deployment_root.axelar_configuration)?,
        )
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Voting verifier contract address");

    solana_deployment_root.voting_verifier = Some(VotingVerifierDeployment {
        code_id,
        address: contract_address.clone(),
        init_params: instantiate_msg,
    });

    Ok(contract_address)
}

fn majority_threshold(config: &AxelarConfiguration) -> axelar_wasm_std::MajorityThreshold {
    axelar_wasm_std::Threshold::try_from(config.voting_verifier_majority_threshould)
        .unwrap()
        .try_into()
        .unwrap()
}

#[tracing::instrument(skip_all)]
pub(crate) async fn init_gateway(
    client: &SigningClient,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<String> {
    use gateway::msg::InstantiateMsg;
    tracing::info!("init gateway");

    let code_id = solana_deployment_root
        .axelar_configuration
        .gateway_code_id
        .ok_or_eyre("gateway code id not present. Was it deployed?")?;
    let instantiate_msg = InstantiateMsg {
        verifier_address: solana_deployment_root
            .voting_verifier
            .as_ref()
            .ok_or_eyre("voting verifier not deployed")?
            .address
            .clone(),
        router_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .router
            .address
            .clone(),
    };
    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("init-gateway".to_string()),
        msg: serde_json::to_vec(&instantiate_msg)?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(
            vec![instantiate.into_any()?],
            &default_gas(&solana_deployment_root.axelar_configuration)?,
        )
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "gateway contract address");

    solana_deployment_root.axelar_gateway = Some(AxelarGatewayDeployment {
        code_id,
        address: contract_address.clone(),
        init_params: instantiate_msg,
    });

    Ok(contract_address)
}

#[tracing::instrument(skip_all, fields(gas = ?config.axelar_chain.gas_price), err)]
pub(crate) fn default_gas(config: &AxelarConfiguration) -> eyre::Result<Gas> {
    // the factual data is in form of "0.123uamplifier" -- we get rid of non-numbers
    let gas_price = config
        .axelar_chain
        .gas_price
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<String>();
    Ok(Gas {
        gas_price: cosmos_client::gas::GasPrice {
            amount: Decimal::from_str_exact(gas_price.as_str())?,
            denom: Denom::from_str(&config.axelar_base_denom)?,
        },
        gas_adjustment: dec!(1.5),
    })
}

#[tracing::instrument(skip_all)]
pub(crate) async fn init_solana_multisig_prover(
    client: &SigningClient,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<String> {
    use crate::cli::cmd::testnet::multisig_prover_api::InstantiateMsg;
    tracing::info!("init multisig prover");

    let code_id = solana_deployment_root
        .axelar_configuration
        .multisig_prover_code_id
        .ok_or_eyre("multisig prover code id not present")?;

    let instantiate_msg = InstantiateMsg {
        admin_address: client.signer_account_id()?.to_string(),
        governance_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .service_registry
            .governance_account
            .clone(),
        gateway_address: solana_deployment_root
            .axelar_gateway
            .as_ref()
            .ok_or_eyre("gateway on Axelar chain not deployed")?
            .address
            .clone(),
        multisig_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .multisig
            .address
            .to_string(),
        coordinator_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .coordinator
            .address
            .to_string(),
        service_registry_address: solana_deployment_root
            .axelar_configuration
            .axelar_chain
            .contracts
            .service_registry
            .address
            .to_string(),
        voting_verifier_address: solana_deployment_root
            .voting_verifier
            .as_ref()
            .ok_or_eyre("voting verifier not deployed?")?
            .address
            .clone(),
        signing_threshold: majority_threshold(&solana_deployment_root.axelar_configuration),
        service_name: solana_deployment_root
            .axelar_configuration
            .service_name
            .to_string(),
        chain_name: solana_deployment_root
            .solana_configuration
            .chain_name_on_axelar_chain
            .to_string(),
        verifier_set_diff_threshold: solana_deployment_root
            .axelar_configuration
            .verifier_set_diff_threshold,
        encoder: solana_deployment_root
            .axelar_configuration
            .multisig_prover_encoder
            .clone(),
        key_type: solana_deployment_root
            .axelar_configuration
            .verifier_key_type,
        domain_separator: hex::encode(solana_deployment_root.solana_configuration.domain_separator),
    };
    tracing::info!(?instantiate_msg, "init msg");

    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("init-multisig-prover".to_string()),
        msg: serde_json::to_vec(&instantiate_msg)?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(
            vec![instantiate.into_any()?],
            &default_gas(&solana_deployment_root.axelar_configuration)?,
        )
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");

    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Multisig prover contract address");

    solana_deployment_root.multisig_prover = Some(MultisigProverDeployment {
        code_id,
        init_params: instantiate_msg,
        address: contract_address.clone(),
    });

    update_verifier_set_multisig_prover(client, solana_deployment_root).await?;

    Ok(contract_address)
}

pub(crate) async fn update_verifier_set_multisig_prover(
    client: &SigningClient,
    solana_deployment_root: &mut SolanaDeploymentRoot,
) -> eyre::Result<()> {
    tracing::info!("calling multisig_prover_api::MultisigProverExecuteMsg::UpdateVerifierSet");
    let msg = multisig_prover_api::MultisigProverExecuteMsg::UpdateVerifierSet {};
    let destination_multisig_prover = cosmrs::AccountId::from_str(
        solana_deployment_root
            .multisig_prover
            .as_ref()
            .ok_or_eyre("multisig prover not deployed")?
            .address
            .as_str(),
    )?;
    let execute = MsgExecuteContract {
        sender: client.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: destination_multisig_prover.clone(),
    };
    let response = client
        .sign_and_broadcast(
            vec![execute.into_any()?],
            &default_gas(&solana_deployment_root.axelar_configuration)?,
        )
        .await?;
    tracing::info!(tx_result = ?response, "raw multisig update verifier set result");
    Ok(())
}

pub(crate) fn domain_separator(chain_name: &str, router_address: &str) -> [u8; 32] {
    hashv(&[chain_name.as_bytes(), router_address.as_bytes()]).to_bytes()
}

pub(crate) fn generate_wallet() -> eyre::Result<()> {
    let key = k256::ecdsa::SigningKey::random(&mut OsRng);
    let key_bytes = key.to_bytes();
    let key_bytes_hex = hex::encode(key_bytes);
    let signing_key = cosmrs::crypto::secp256k1::SigningKey::new(Box::new(key));
    let account_id = signing_key.public_key().account_id("axelar")?;
    tracing::info!(
        account_id = ?account_id,
        private_key = ?key_bytes_hex,
        "generated a new private key, fund it according to the docs here - https://docs.axelar.dev/validator/amplifier/verifier-onboarding#fund-your-wallet"
    );
    Ok(())
}

fn read_wasm_for_deployment(wasm_artifact_name: &str) -> eyre::Result<Vec<u8>> {
    let wasm = path::optimised_wasm_output(wasm_artifact_name);
    let wasm = std::fs::read(wasm)?;
    let mut output = Vec::with_capacity(wasm.len());
    flate2::write::GzEncoder::new(&mut output, flate2::Compression::best())
        .write_all(&wasm)
        .unwrap();
    tracing::info!(bytes = output.len(), "wasm module found");
    Ok(output)
}

pub(crate) mod ampd {

    use std::thread;

    use inquire::Confirm;
    use tracing::info;
    use xshell::Shell;

    use super::path::axelar_amplifier_dir;
    use crate::cli::cmd::cosmwasm::path::{self, ampd_home_dir};
    use crate::cli::cmd::deployments::SolanaDeploymentRoot;
    use crate::cli::cmd::path::{workspace_root_dir, xtask_crate_root_dir};

    pub(crate) async fn setup_ampd(deployment_root: &SolanaDeploymentRoot) -> eyre::Result<()> {
        if !Confirm::new("Welcome to ampd-setup ! This will perform/guide you through the verifier onboarding process described here https://docs.axelar.dev/validator/amplifier/verifier-onboarding (devnet-amplifier chain).

        It will overwrite your $HOME/.ampd/config.toml if it exist.

        Do you want to continue  ?").prompt()? {
            return Ok(println!("Cannot continue without user confirmation."))
        }

        build_ampd()?;
        let sh = xshell::Shell::new()?;
        let _dir = sh.push_dir(axelar_amplifier_dir());
        let ampd_build_path = path::ampd_bin();

        info!("Copying Solana ampd configuration template to $HOME/.ampd/config.toml");
        tokio::fs::create_dir_all(ampd_home_dir()).await?;
        tokio::fs::copy(
            xtask_crate_root_dir().join("ampd-config.toml"),
            ampd_home_dir().join("config.toml"),
        )
        .await?;

        if !Confirm::new("Now we need to bring up tofnd service. Is it already running ?").with_help_message("You can easily run it by executing:
        docker run -p 50051:50051 --env MNEMONIC_CMD=auto --env NOPASSWORD=true -v ./tofnd:/.tofnd haiyizxx/tofnd:latest

        MAKE SURE TO PLACE YOUR SEED AT ./tofnd/import BEFORE EXECUTING AND CHECK PERMISSIONS if you have one").prompt()? {
            return Ok(println!("Cannot continue without a running instance of tofnd."))
        }

        let verifier_address = String::from_utf8(
            sh.cmd(&ampd_build_path)
                .args(vec!["verifier-address"])
                .output()?
                .stdout,
        )?;

        let verifier_address = verifier_address.split("address: ").collect::<Vec<&str>>();

        let verifier_address = verifier_address
            .get(1)
            .expect("We should be able to parse an address from this output");

        if !Confirm::new("Is the ampd verifier address funded ?")
            .with_help_message(&format!(
                "It can be easily funded it by requesting tokens in the faucet discord channel:
        https://discord.com/channels/770814806105128977/1002423218772136056/1217885883152334918

        Just write there:

        !faucet devnet-amplifier {verifier_address}

        ",
            ))
            .prompt()?
        {
            return Ok(println!("Cannot continue without a funded ampd address."));
        }

        info!("Bonding ampd verifier ...");
        sh.cmd(&ampd_build_path)
            .args(vec![
                "bond-verifier",
                deployment_root.axelar_configuration.service_name.as_str(),
                "100",
                "uamplifier",
            ])
            .run()?;

        info!("Registering ampd public key ...");
        let _err = sh
            .cmd(&ampd_build_path)
            .args(vec!["register-public-key", "ecdsa"])
            .run()
            .inspect_err(|err| {
                tracing::error!(?err, "error in registering the public key");
            });

        info!("Registering support for Solana blockchain ...");
        sh.cmd(&ampd_build_path)
            .args(vec![
                "register-chain-support",
                deployment_root.axelar_configuration.service_name.as_str(),
                deployment_root
                    .solana_configuration
                    .chain_name_on_axelar_chain
                    .as_str(),
            ])
            .run()?;

        if !Confirm::new("Is the new ampd validator already authorized ?").with_help_message("You can do it by filling this form:
            https://docs.google.com/forms/d/e/1FAIpQLSfQQhk292yT9j8sJF5ARRIE8PpI3LjuFc8rr7xZW7posSLtJA/viewform").prompt()? {
                return Ok(println!("Cannot continue without a running instance of tofnd."))
            }

        println!(
            "We are ready to go ! just execute ampd by: {}",
            &ampd_build_path.to_string_lossy()
        );

        Ok(())
    }

    fn build_ampd() -> eyre::Result<()> {
        let sh = Shell::new()?;
        let _dir = sh.push_dir(axelar_amplifier_dir());
        info!("Compiling ampd ...");
        sh.cmd("cargo").args(vec!["build", "-p", "ampd"]).run()?;
        Ok(())
    }

    pub(crate) async fn start_with_tofnd() -> eyre::Result<()> {
        build_ampd()?;

        tracing::info!("starting tofnd");
        let expected_import_file = workspace_root_dir().join("tofnd").join("import");
        if !expected_import_file.try_exists()? {
            eyre::bail!("create a new `tofnd/import` file that would contain the tofnd root seed!")
        }

        let tofnd_process = thread::spawn(move || {
            let container_name = "tofnd-solana";
            let sh = Shell::new()?;
            // Check if the container exists (running or not)
            let output = sh
                .cmd("docker")
                .args([
                    "ps",
                    "-a",
                    "--filter",
                    format!("name={container_name}").as_str(),
                    "--format",
                    "{{.Names}} {{.Status}}",
                ])
                .read()
                .expect("Failed to execute command");
            tracing::info!(output, "docker tofnd check output");

            if output.trim().is_empty() {
                // Container does not exist, create and start it
                let _ws = sh.push_dir(workspace_root_dir());

                let tofnd = sh.cmd("docker").args([
                    "run",
                    "-d",
                    "--name",
                    container_name,
                    "-p",
                    "50051:50051",
                    "--env",
                    "MNEMONIC_CMD=auto",
                    "--env",
                    "NOPASSWORD=true",
                    "-v",
                    "./tofnd:/.tofnd",
                    "haiyizxx/tofnd:latest",
                ]);
                tofnd.run()?;
                tracing::info!("Created and started container {}", container_name);
            } else {
                // Container exists
                let status_line = output.trim();
                if status_line.contains("Up") {
                    // Container is running
                    tracing::info!("Container {} is already running", container_name);
                } else {
                    // Container is not running, start it
                    let start = sh.cmd("docker").args(["start", container_name]);
                    start.run()?;
                    tracing::info!("Started container {}", container_name);
                }
            }
            Ok::<_, eyre::Error>(())
        });

        // sleep for 5 secs to allow tofnd to spawn
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        tofnd_process.join().unwrap()?;

        // spawn ampd
        tracing::info!("spawning ampd");
        let sh = Shell::new()?;
        sh.cmd(path::ampd_bin()).run()?;

        Ok(())
    }
}

pub(crate) mod path {
    use std::path::PathBuf;

    use crate::cli::cmd::path::{home_dir, workspace_root_dir};

    pub(crate) fn axelar_amplifier_dir() -> PathBuf {
        let workspace_root = workspace_root_dir();
        let root_dir = workspace_root.parent().unwrap();
        root_dir.join("axelar-amplifier")
    }

    pub(crate) fn wasm_opt_binary() -> PathBuf {
        binaryen_unpacked()
            .join("binaryen-version_117")
            .join("bin")
            .join("wasm-opt")
    }

    pub(crate) fn ampd_home_dir() -> PathBuf {
        home_dir().join(".ampd")
    }

    pub(crate) fn binaryen_tar_file() -> PathBuf {
        workspace_root_dir().join("target").join("binaryen.tar.gz")
    }

    pub(crate) fn binaryen_unpacked() -> PathBuf {
        workspace_root_dir().join("target").join("binaryen")
    }

    pub(crate) fn optimised_wasm_output(contract_name: &str) -> PathBuf {
        axelar_amplifier_dir()
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{contract_name}.optimised.wasm"))
    }
    pub(crate) fn ampd_bin() -> PathBuf {
        axelar_amplifier_dir()
            .join("target")
            .join("debug")
            .join("ampd")
    }
}

pub(crate) trait ResponseEventExtract {
    fn extract(&self, event: &str, attribute: &str) -> eyre::Result<String>;
}

impl ResponseEventExtract for cosmrs::rpc::endpoint::broadcast::tx_commit::Response {
    fn extract(&self, event: &str, attribute: &str) -> eyre::Result<String> {
        use base64::prelude::{Engine as _, BASE64_STANDARD};

        let encoded_attribute = BASE64_STANDARD.encode(attribute);

        let value = self
            .tx_result
            .events
            .iter()
            .find(|e| e.kind == event)
            .ok_or_eyre("Event not found")?
            .attributes
            .iter()
            .find(|a| a.key == encoded_attribute)
            .ok_or_eyre("Attribute not found")?
            .value
            .clone();

        let value_bytes = BASE64_STANDARD.decode(value)?;
        let result = String::from_utf8(value_bytes)?;

        Ok(result)
    }
}
