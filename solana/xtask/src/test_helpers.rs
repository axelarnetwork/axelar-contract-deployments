use xshell::{cmd, Shell};

use crate::cli::cmd::path::workspace_root_dir;

pub(crate) fn build_gateway_contract() {
    let contract_dir = workspace_root_dir().join("programs").join("gateway");
    let sh = Shell::new().unwrap();
    sh.change_dir(contract_dir);
    cmd!(sh, "cargo build-bpf").run().unwrap();
}
