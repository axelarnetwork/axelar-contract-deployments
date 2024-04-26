use std::path::PathBuf;

use ethers::contract::Abigen;

/// Using a buid.rs script to generate the bindings for the EVM contracts
/// and write them to the OUT_DIR.
///
/// The generated bindings are then included in the lib.rs file.
/// This is preffered to using the `abigen!` macro because it does not act on
/// `*.json` file changes and cannot be used in a `build.rs` script.
fn main() {
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
            "AxelarGateway",
            "../../../evm-contracts/out/AxelarGateway.sol/AxelarGateway.json",
        ),
        (
            "AxelarAuthWeighted",
            "../../../evm-contracts/out/AxelarAuthWeighted.sol/AxelarAuthWeighted.json",
        ),
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
