use std::io::Write;
use std::str::FromStr;

use cosmrs::cosmwasm::MsgStoreCode;
use cosmrs::crypto::secp256k1::SigningKey;
use cosmrs::tx::Msg;
use cosmrs::Denom;
use eyre::OptionExt;
use k256::elliptic_curve::rand_core::OsRng;
use rust_decimal_macros::dec;
use xshell::Shell;

mod build;
mod cosmos_client;

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

const AXELAR_DEVNET: Network = Network {
    chain_id: "devnet-amplifier",
    grpc_endpoint: "http://devnet-amplifier.axelar.dev:9090",
    rpc_endpoint: "http://devnet-amplifier.axelar.dev:26657",
};

const AXELAR_ACCOUNT_PREFIX: &str = "axelar";
const AXELAR_BASE_DENOM: &str = "uamplifier";

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

pub(crate) async fn deploy(signing_key: SigningKey) -> eyre::Result<()> {
    let network = AXELAR_DEVNET.clone();
    let client = SigningClient {
        network: network.clone(),
        account_prefix: AXELAR_ACCOUNT_PREFIX.to_owned(),
        signing_key,
    };

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
        let root_dir = workspace_root_dir();
        root_dir.join("axelar-amplifier")
    }

    pub(crate) fn wasm_opt_binary() -> PathBuf {
        binaryen_unpacked()
            .join("binaryen-version_117")
            .join("bin")
            .join("wasm-opt")
    }

    pub(crate) fn binaryen_tar_file() -> PathBuf {
        PathBuf::from_iter(["target", "binaryen.tar.gz"])
    }

    pub(crate) fn binaryen_unpacked() -> PathBuf {
        PathBuf::from_iter(["target", "binaryen"])
    }

    pub(crate) fn optimised_wasm_output(contract_name: &str) -> PathBuf {
        axelar_amplifier_dir()
            .join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{contract_name}.optimised.wasm"))
    }
}

trait ResponseEventExtract {
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
