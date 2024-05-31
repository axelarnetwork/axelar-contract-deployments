use std::env;
use std::path::PathBuf;

use xshell::{cmd, Shell};

pub fn build_gateway_contract() {
    let contract_dir = workspace_root().join("programs").join("gateway");
    let sh = Shell::new().unwrap();
    sh.change_dir(contract_dir);
    cmd!(sh, "cargo build-bpf").run().unwrap();
}

pub fn workspace_root() -> PathBuf {
    let dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
