use std::io::Write;
use std::str::FromStr;

use cosmrs::cosmwasm::{MsgExecuteContract, MsgInstantiateContract, MsgStoreCode};
use cosmrs::tx::Msg;
use cosmrs::Denom;
use eyre::OptionExt;
use k256::elliptic_curve::rand_core::OsRng;
use multisig::key::KeyType;
use rust_decimal_macros::dec;
use solana_sdk::keccak::hashv;
use xshell::Shell;

mod build;
pub(crate) mod cosmos_client;

use build::{build_contracts, download_wasm_opt, setup_toolchain, unpack_tar_gz};

use self::cosmos_client::network::Network;
use self::path::{binaryen_tar_file, binaryen_unpacked, wasm_opt_binary};
use crate::cli::cmd::cosmwasm::cosmos_client::gas::Gas;
use crate::cli::cmd::cosmwasm::cosmos_client::signer::SigningClient;
use crate::cli::cmd::testnet::{multisig_prover_api, solana_domain_separator, SOLANA_CHAIN_NAME};

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

pub(crate) const AXELAR_DEVNET: Network = Network {
    chain_id: "devnet-amplifier",
    grpc_endpoint: "http://devnet-amplifier.axelar.dev:9090",
    rpc_endpoint: "http://devnet-amplifier.axelar.dev:26657",
};

pub(crate) const AXELAR_ACCOUNT_PREFIX: &str = "axelar";

const AXELAR_BASE_DENOM: &str = "uamplifier";
const ROUTER_ADDRESS: &str = "axelar14jjdxqhuxk803e9pq64w4fgf385y86xxhkpzswe9crmu6vxycezst0zq8y";
const GOVERNANCE_ADDRESS: &str = "axelar1zlr7e5qf3sz7yf890rkh9tcnu87234k6k7ytd9";
const MULTISIG_ADDRESS: &str = "axelar19jxy26z0qnnspa45y5nru0l5rmy9d637z5km2ndjxthfxf5qaswst9290r";
const COORDINATOR_ADDRESS: &str =
    "axelar1m2498n4h2tskcsmssjnzswl5e6eflmqnh487ds47yxyu6y5h4zuqr9zk4g";
const SERVICE_REGISTRY_ADDRESS: &str =
    "axelar1c9fkszt5lq34vvvlat3fxj6yv7ejtqapz04e97vtc9m5z9cwnamq8zjlhz";
const BLOCK_EXPIRY: u64 = 10;
const CONFIRMATION_HEIGHT: u64 = 1;
const REWARDS_ADDRESS: &str = "axelar1vaj9sfzc3z0gpel90wu4ljutncutv0wuhvvwfsh30rqxq422z89qnd989l";
const SERVICE_NAME: &str = "validators";
const VERIFIER_SET_DIFF_THRESHOLD: u32 = 1;

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

pub(crate) async fn deploy(client: &SigningClient) -> eyre::Result<[u64; 3]> {
    // deploy each contract - do not instantiate
    let mut code_ids = [0_u64; 3];
    for (contract, code_id_storage) in CONTRACTS.into_iter().zip(code_ids.iter_mut()) {
        tracing::info!(contract = ?contract.wasm_artifact_name, "about to deploy contract");

        let wasm_byte_code = read_wasm_for_deployment(contract.wasm_artifact_name)?;
        let msg_store_code = MsgStoreCode {
            sender: client.signer_account_id()?,
            wasm_byte_code,
            instantiate_permission: None,
        }
        .to_any()?;

        let response = client
            .sign_and_broadcast(
                vec![msg_store_code],
                &Gas {
                    gas_price: cosmos_client::gas::GasPrice {
                        amount: dec!(0.007),
                        denom: Denom::from_str(AXELAR_BASE_DENOM)
                            .expect("base denom is always valid"),
                    },
                    gas_adjustment: dec!(1.5),
                },
            )
            .await?;
        tracing::debug!(tx_result = ?response, "raw response reult");

        let code_id = response.extract("store_code", "code_id")?;
        tracing::info!(code_id, contract = ?contract.wasm_artifact_name, "code stored");
        let code_id = code_id.parse()?;
        *code_id_storage = code_id;
    }
    Ok(code_ids)
}

pub(crate) async fn init_solana_voting_verifier(
    code_id: u64,
    client: &SigningClient,
) -> eyre::Result<String> {
    use voting_verifier::msg::InstantiateMsg;

    let instantiate_msg = InstantiateMsg {
        service_registry_address: SERVICE_REGISTRY_ADDRESS.to_string().try_into().unwrap(),
        service_name: SERVICE_NAME.to_string().try_into().unwrap(),
        source_gateway_address: gmp_gateway::id().to_string().try_into().unwrap(),
        voting_threshold: majority_threshold(),
        block_expiry: BLOCK_EXPIRY,
        confirmation_height: CONFIRMATION_HEIGHT,
        source_chain: SOLANA_CHAIN_NAME.to_string().try_into().unwrap(),
        rewards_address: REWARDS_ADDRESS.to_string(),
        governance_address: GOVERNANCE_ADDRESS.to_string().try_into().unwrap(),
        msg_id_format:
            axelar_wasm_std::msg_id::MessageIdFormat::Base58SolanaTxSignatureAndEventIndex,
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
        .sign_and_broadcast(vec![instantiate.into_any()?], &default_gas())
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Voting verifier contract address");

    Ok(contract_address)
}

fn majority_threshold() -> axelar_wasm_std::MajorityThreshold {
    axelar_wasm_std::Threshold::try_from((1u64, 1u64))
        .unwrap()
        .try_into()
        .unwrap()
}

pub(crate) async fn init_gateway(
    code_id: u64,
    client: &SigningClient,
    voting_verifier_address: String,
) -> eyre::Result<String> {
    use gateway::msg::InstantiateMsg;

    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("init-gateway".to_string()),
        msg: serde_json::to_vec(&InstantiateMsg {
            verifier_address: voting_verifier_address,
            router_address: ROUTER_ADDRESS.to_string(),
        })?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(vec![instantiate.into_any()?], &default_gas())
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "gateway contract address");

    Ok(contract_address)
}

pub(crate) fn default_gas() -> Gas {
    Gas {
        gas_price: cosmos_client::gas::GasPrice {
            amount: dec!(0.007),
            denom: Denom::from_str(AXELAR_BASE_DENOM).expect("base denom is always valid"),
        },
        gas_adjustment: dec!(1.5),
    }
}

pub(crate) async fn init_solana_multisig_prover(
    code_id: u64,
    client: &SigningClient,
    gateway_address: String,
    voting_verifier_address: String,
) -> eyre::Result<String> {
    use crate::cli::cmd::testnet::multisig_prover_api::InstantiateMsg;

    let msg = InstantiateMsg {
        admin_address: client.signer_account_id()?.to_string(),
        governance_address: GOVERNANCE_ADDRESS.to_string(),
        gateway_address,
        multisig_address: MULTISIG_ADDRESS.to_string(),
        coordinator_address: COORDINATOR_ADDRESS.to_string(),
        service_registry_address: SERVICE_REGISTRY_ADDRESS.to_string(),
        voting_verifier_address,
        signing_threshold: majority_threshold(),
        service_name: SERVICE_NAME.to_string(),
        chain_name: SOLANA_CHAIN_NAME.to_string(),
        verifier_set_diff_threshold: VERIFIER_SET_DIFF_THRESHOLD,
        encoder: "rkyv".to_string(),
        key_type: KeyType::Ecdsa,
        domain_separator: hex::encode(solana_domain_separator()),
    };
    tracing::info!(?msg, "init msg");

    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("init-multisig-prover".to_string()),
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(vec![instantiate.into_any()?], &default_gas())
        .await?;
    tracing::debug!(tx_result = ?response, "raw response reult");

    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Multisig prover contract address");

    update_verifier_set_multisig_prover(contract_address.as_str(), client).await?;
    Ok(contract_address)
}

pub(crate) async fn update_verifier_set_multisig_prover(
    contract_address: &str,
    client: &SigningClient,
) -> eyre::Result<()> {
    tracing::info!("calling multisig_prover_api::MultisigProverExecuteMsg::UpdateVerifierSet");
    let msg = multisig_prover_api::MultisigProverExecuteMsg::UpdateVerifierSet {};
    let destination_multisig_prover = cosmrs::AccountId::from_str(contract_address).unwrap();
    let execute = MsgExecuteContract {
        sender: client.signer_account_id()?,
        msg: serde_json::to_vec(&msg)?,
        funds: vec![],
        contract: destination_multisig_prover.clone(),
    };
    let response = client
        .sign_and_broadcast(vec![execute.into_any()?], &default_gas())
        .await?;
    tracing::info!(tx_result = ?response, "raw multisig update verifier set result");
    Ok(())
}

pub(crate) fn domain_separator(chain_name: &str, chain_id: u64) -> [u8; 32] {
    hashv(&[
        chain_name.as_bytes(),
        ROUTER_ADDRESS.as_bytes(),
        &chain_id.to_le_bytes(),
    ])
    .to_bytes()
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
    use crate::cli::cmd::path::{workspace_root_dir, xtask_crate_root_dir};
    use crate::cli::cmd::testnet::SOLANA_CHAIN_NAME;

    pub(crate) async fn setup_ampd() -> eyre::Result<()> {
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
            .args(vec!["bond-verifier", "validators", "100", "uamplifier"])
            .run()?;

        info!("Registering ampd public key ...");
        sh.cmd(&ampd_build_path)
            .args(vec!["register-public-key", "ecdsa"])
            .run()?;

        info!("Registering support for Solana blockchain ...");
        sh.cmd(&ampd_build_path)
            .args(vec![
                "register-chain-support",
                "validators",
                SOLANA_CHAIN_NAME,
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
        let tofnd_process = thread::spawn(move || {
            // Run the docker ps command with filtering by the container name
            let container_name = "tofnd-solana";
            let sh = Shell::new()?;
            let output = sh
                .cmd("docker")
                .args([
                    "ps",
                    "--filter",
                    format!("name={container_name}").as_str(),
                    "--format",
                    "{{.Names}}",
                ])
                .read()
                .expect("Failed to execute command");
            tracing::info!(output, "docker tofnd check output");

            // Check if the output contains the container name
            if output.contains(container_name) {
                println!("Container {container_name} is running");
                return Ok(());
            }
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
