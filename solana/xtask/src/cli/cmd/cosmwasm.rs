use xshell::Shell;

mod build;

use build::contract::build_contracts;
use build::download::download_wasm_opt;
use build::toolchain::setup_toolchain;
use build::unpack::unpack_tar_gz;

use self::path::{binaryen_tar_file, binaryen_unpacked, wasm_opt_binary};

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

pub(crate) async fn build() -> anyhow::Result<()> {
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
}
