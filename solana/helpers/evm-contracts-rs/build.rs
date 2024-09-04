use std::path::PathBuf;

use ethers::contract::Abigen;

/// Using a build.rs script to generate the bindings for the EVM contracts
/// and write them to the OUT_DIR.
///
/// The generated bindings are then included in the lib.rs file.
/// This is preferred to using the `abigen!` macro because it does not act on
/// `*.json` file changes and cannot be used in a `build.rs` script.
fn main() {
    build_contract();
    let output_dir = std::env::var("OUT_DIR").unwrap();
    let output_dir = PathBuf::from(output_dir);

    let contracts = [
        (
            "ExampleEncoder",
            "../../../evm-contracts/out/ExampleEncoder.sol/ExampleSolanaGatewayEncoder.json",
        ),
        (
            "AxelarMemo",
            "../../../evm-contracts/out/AxelarMemo.sol/AxelarMemo.json",
        ),
        (
            "AxelarAmplifierGateway",
            "../../../evm-contracts/out/AxelarAmplifierGateway.sol/AxelarAmplifierGateway.json",
        ),
        (
            "AxelarAmplifierGatewayProxy",
            "../../../evm-contracts/out/AxelarAmplifierGatewayProxy.sol/AxelarAmplifierGatewayProxy.json",
        )
    ];
    for (contract_name, path) in contracts {
        let mut output = output_dir.clone();
        output.push(format!("{}.rs", contract_name));

        Abigen::new(contract_name, path)
            .unwrap()
            .emit_cargo_directives(true)
            .generate()
            .unwrap()
            .write_to_file(output.as_path())
            .unwrap();
    }

    println!("cargo:rerun-if-changed=build.rs");
}

fn build_contract() {
    let root_dir = workspace_root_dir();
    let contract_dir = root_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("evm-contracts");
    let sh = xshell::Shell::new().unwrap();
    sh.change_dir(contract_dir);
    xshell::cmd!(sh, "forge build")
        .run()
        .expect("do you have `foundry` installed?");
}

fn workspace_root_dir() -> PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
