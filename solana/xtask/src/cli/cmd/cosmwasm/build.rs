pub(crate) use contract::build_contracts;
pub(crate) use download::download_wasm_opt;
pub(crate) use toolchain::setup_toolchain;
pub(crate) use unpack::unpack_tar_gz;

pub(crate) mod contract {
    use std::path::Path;

    use eyre::Result;
    use xshell::{cmd, Shell};

    use crate::cli::cmd::cosmwasm::path::{axelar_amplifier_dir, optimised_wasm_output};
    use crate::cli::cmd::cosmwasm::WasmContracts;

    pub(crate) async fn build_contracts(
        sh: &Shell,
        wasm_opt: &Path,
        contracts: &[WasmContracts],
    ) -> Result<()> {
        let amplifer_dir = axelar_amplifier_dir();
        let _env_guard = sh.push_env("RUSTFLAGS", "-C link-args=-s");

        for contract in contracts {
            let contract_dir = amplifer_dir
                .join("contracts")
                .join(contract.contract_project_folder);

            tracing::info!(contract_dir = ?contract_dir, "preparing to process cosmwasm contract");
            let in_contract_dir = sh.push_dir(contract_dir.clone());

            tracing::info!("building contract");
            cmd!(sh, "cargo wasm").run()?;

            let wasm_artifact = amplifer_dir
                .join("target")
                .join("wasm32-unknown-unknown")
                .join("release")
                .join(format!("{}.wasm", contract.wasm_artifact_name));
            let wasm_artifact_optimised = optimised_wasm_output(contract.wasm_artifact_name);

            drop(in_contract_dir);
            tracing::info!("applying optimiser");
            cmd!(
                sh,
                "{wasm_opt} -Oz --signext-lowering {wasm_artifact} -o {wasm_artifact_optimised}"
            )
            .run()?;
        }

        Ok(())
    }
}

pub(crate) mod toolchain {
    use eyre::Result;
    use xshell::{cmd, PushEnv, Shell};

    use crate::cli::cmd::cosmwasm::path::axelar_amplifier_dir;
    // install the cosmwasm target for the amplifier toolchain.
    // because the amplifier submodule is under the same file tree in the `/solana`
    // workspace, the `solana/rust-toolchain.toml` override carries over.
    //
    // It just so happens that our nightly cannot compile the contracts, but stable
    // cannot either. see: https://users.rust-lang.org/t/error-e0635-unknown-feature-stdsimd/106445/2
    // therefore we need to use an *old* version of nightly
    //
    // TODO: we should upstream a `rust-toolchain.toml` contribution to the
    // amplifier repo
    pub(crate) fn setup_toolchain(sh: &Shell) -> Result<PushEnv<'_>> {
        let amplifer_dir = axelar_amplifier_dir();
        let in_ampl_dir = sh.push_dir(amplifer_dir.clone());
        cmd!(sh, "rustup install nightly-2024-02-01").run()?;
        let env_toolchain = sh.push_env("RUSTUP_TOOLCHAIN", "nightly-2024-02-01");
        cmd!(sh, "rustup target add wasm32-unknown-unknown").run()?;
        drop(in_ampl_dir);

        Ok(env_toolchain)
    }
}

pub(crate) mod unpack {
    use std::fs::create_dir_all;
    use std::path::Path;

    use eyre::Result;
    use flate2::read::GzDecoder;
    use tar::Archive;

    pub(crate) fn unpack_tar_gz(file_path: &Path, output_dir: &Path) -> Result<()> {
        let file = std::fs::File::open(file_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        create_dir_all(output_dir)?;
        archive.unpack(output_dir)?;

        tracing::info!(output_dir = ?output_dir, "Unpacked successfully");

        Ok(())
    }
}

pub(crate) mod download {
    use std::io::Write;
    use std::path::Path;

    use eyre::Result;
    use futures::StreamExt;

    pub(crate) async fn download_wasm_opt(file_path: &Path) -> Result<()> {
        let url = determine_download_url();
        let client = reqwest::Client::new();
        let response = client.get(url).send().await?;
        if !response.status().is_success() {
            tracing::error!(status = ?response.status(), "Failed to download file");
            eyre::bail!("failed");
        }

        let mut file = std::fs::File::create(file_path)?;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
        }

        tracing::info!(file_path = ?file_path, "Downloaded successfully to");
        Ok(())
    }

    pub(crate) fn determine_download_url() -> &'static str {
        const DOWNLOAD_BASE: &str =
            "https://github.com/WebAssembly/binaryen/releases/download/version_117/binaryen-version_117-";
        const SUFFIX: &str = if cfg!(target_os = "linux") && cfg!(target_arch = "x86_64") {
            "x86_64-linux.tar.gz"
        } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
            "aarch64-linux.tar.gz"
        } else if cfg!(target_os = "macos") && cfg!(target_arch = "x86_64") {
            "x86_64-macos.tar.gz"
        } else if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
            "arm64-macos.tar.gz"
        } else if cfg!(target_os = "windows") && cfg!(target_arch = "x86_64") {
            "x86_64-windows.tar.gz"
        } else {
            panic!("Unsupported OS/Architecture combination");
        };
        const_format::concatcp!(DOWNLOAD_BASE, SUFFIX)
    }
}
