use std::io::Write;
use std::str::FromStr;

use axelar_wasm_std::MajorityThreshold;
use cosmrs::cosmwasm::{MsgInstantiateContract, MsgStoreCode};
use cosmrs::tx::Msg;
use cosmrs::Denom;
use eyre::OptionExt;
use k256::elliptic_curve::rand_core::OsRng;
use multisig::key::KeyType;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use solana_sdk::keccak::hashv;
use xshell::Shell;

mod build;
pub(crate) mod cosmos_client;

use build::{build_contracts, download_wasm_opt, setup_toolchain, unpack_tar_gz};

use self::cosmos_client::network::Network;
use self::path::{binaryen_tar_file, binaryen_unpacked, wasm_opt_binary};
use crate::cli::cmd::cosmwasm::cosmos_client::gas::Gas;
use crate::cli::cmd::cosmwasm::cosmos_client::signer::SigningClient;

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

pub(crate) async fn deploy(client: SigningClient) -> eyre::Result<()> {
    // deploy each contract - do not instantiate
    for contract in CONTRACTS {
        tracing::info!(contract = ?contract.wasm_artifact_name, "about to deploy contract");

        let wasm_byte_code = read_wasm_for_delpoyment(contract.wasm_artifact_name)?;
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
        tracing::debug!(tx_result = ?response, "raw respones reult");

        let code_id = response.extract("store_code", "code_id")?;
        tracing::info!(code_id, contract = ?contract.wasm_artifact_name, "code stored");
    }
    Ok(())
}

pub(crate) async fn init_voting_verifier(
    code_id: u64,
    client: SigningClient,
    source_chain: String,
) -> eyre::Result<()> {
    use voting_verifier::msg::InstantiateMsg;

    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("voting-verifier".to_string()),
        msg: serde_json::to_vec(&InstantiateMsg {
            service_registry_address: SERVICE_REGISTRY_ADDRESS.to_string().try_into().unwrap(),
            service_name: SERVICE_NAME.to_string().try_into().unwrap(),
            source_gateway_address: gmp_gateway::id().to_string().try_into().unwrap(),
            voting_threshold: majority_threshold(),
            block_expiry: BLOCK_EXPIRY,
            confirmation_height: CONFIRMATION_HEIGHT,
            source_chain: source_chain.to_string().try_into().unwrap(),
            rewards_address: REWARDS_ADDRESS.to_string(),
            governance_address: GOVERNANCE_ADDRESS.to_string().try_into().unwrap(),
            msg_id_format: axelar_wasm_std::msg_id::MessageIdFormat::Base58TxDigestAndEventIndex,
        })?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(vec![instantiate.into_any()?], &default_gas())
        .await?;
    tracing::debug!(tx_result = ?response, "raw respones reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Voting verifier contract address");

    Ok(())
}

fn majority_threshold() -> axelar_wasm_std::MajorityThreshold {
    axelar_wasm_std::Threshold::try_from((1u64, 1u64))
        .unwrap()
        .try_into()
        .unwrap()
}

pub(crate) async fn init_gateway(
    code_id: u64,
    client: SigningClient,
    voting_verifier_address: String,
) -> eyre::Result<()> {
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
    tracing::debug!(tx_result = ?response, "raw respones reult");
    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "gateway contract address");

    Ok(())
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

pub(crate) async fn init_multisig_prover(
    code_id: u64,
    client: SigningClient,
    chain_id: u64,
    gateway_address: String,
    voting_verifier_address: String,
    chain_name: String,
) -> eyre::Result<()> {
    // NOTE: there are issues with using `multisig-prover` as a dependency (bulid
    // breaks)
    #[derive(Serialize, Deserialize)]
    pub(crate) struct InstantiateMsg {
        admin_address: String,
        governance_address: String,
        gateway_address: String,
        multisig_address: String,
        coordinator_address: String,
        service_registry_address: String,
        voting_verifier_address: String,
        signing_threshold: MajorityThreshold,
        service_name: String,
        chain_name: String,
        verifier_set_diff_threshold: u32,
        encoder: String,
        key_type: KeyType,
        domain_separator: String,
    }

    let domain_separator = domain_separator(&chain_name, chain_id);
    let instantiate = MsgInstantiateContract {
        sender: client.signer_account_id()?,
        admin: Some(client.signer_account_id()?),
        code_id,
        label: Some("init-multisig-prover".to_string()),
        msg: serde_json::to_vec(&InstantiateMsg {
            admin_address: client.signer_account_id()?.to_string(),
            governance_address: GOVERNANCE_ADDRESS.to_string(),
            gateway_address,
            multisig_address: MULTISIG_ADDRESS.to_string(),
            coordinator_address: COORDINATOR_ADDRESS.to_string(),
            service_registry_address: SERVICE_REGISTRY_ADDRESS.to_string(),
            voting_verifier_address,
            signing_threshold: majority_threshold(),
            service_name: SERVICE_NAME.to_string(),
            chain_name: chain_name.clone(),
            verifier_set_diff_threshold: VERIFIER_SET_DIFF_THRESHOLD,
            // todo change to rkyv encoding scheme once the multisig-prover supports it
            encoder: "abi".to_string(),
            key_type: KeyType::Ecdsa,
            domain_separator,
        })?,
        funds: vec![],
    };
    let response = client
        .sign_and_broadcast(vec![instantiate.into_any()?], &default_gas())
        .await?;
    tracing::debug!(tx_result = ?response, "raw respones reult");

    let contract_address = response.extract("instantiate", "_contract_address")?;
    tracing::info!(contract_address, "Multisig prover contract address");

    Ok(())
}

fn domain_separator(chain_name: &str, chain_id: u64) -> String {
    let domain_separator = hashv(&[
        chain_name.as_bytes(),
        ROUTER_ADDRESS.as_bytes(),
        &chain_id.to_le_bytes(),
    ])
    .to_bytes();
    hex::encode(domain_separator)
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
        "genereted a new private key, fund it according to the docs here - https://docs.axelar.dev/validator/amplifier/verifier-onboarding#fund-your-wallet"
    );
    Ok(())
}

fn read_wasm_for_delpoyment(wasm_artifact_name: &str) -> eyre::Result<Vec<u8>> {
    let wasm = path::optimised_wasm_output(wasm_artifact_name);
    let wasm = std::fs::read(wasm)?;
    let mut output = Vec::with_capacity(wasm.len());
    flate2::write::GzEncoder::new(&mut output, flate2::Compression::best())
        .write_all(&wasm)
        .unwrap();
    tracing::info!(bytes = output.len(), "wasm module found");
    Ok(output)
}

pub(crate) mod path {
    use std::path::PathBuf;

    use crate::cli::cmd::path::workspace_root_dir;

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
