use std::collections::BTreeMap;
use std::io::Write;
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use axelar_rkyv_encoding::types::{PublicKey, VerifierSet};
use borsh::from_slice;
use clap::Parser;
use gmp_gateway::axelar_auth_weighted::AxelarAuthWeighted;
use gmp_gateway::state::GatewayConfig;
use serial_test::serial;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_test_validator::{TestValidatorGenesis, UpgradeableProgramInfo};
use tempfile::NamedTempFile;

use crate::cli::cmd::solana::SolanaContract;
use crate::cli::cmd::{self};
use crate::cli::Cli;
use crate::test_helpers::build_gateway_contract;

#[tokio::test]
#[serial]
async fn build_actually_works() {
    // setup
    let args = vec!["xtask", "solana", "build", "gmp-gateway"];
    let cli: Cli = Cli::try_parse_from(args).unwrap();

    // action
    let result = cli.run().await;

    // assert
    assert!(result.is_ok());
}

#[test_log::test(tokio::test)]
#[serial]
async fn deploy_actually_works() {
    // setup
    solana_logger::setup_with_default("solana_program_runtime=warn");
    let validator = TestValidatorGenesis::default();
    let (validator, keypair) = validator.start_async().await;
    validator.set_startup_verification_complete_for_tests();
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{:?}", keypair.to_bytes()).unwrap();

    // setup programid
    let program_id = Keypair::new();
    let mut file_program_id = NamedTempFile::new().unwrap();
    write!(file_program_id, "{:?}", program_id.to_bytes()).unwrap();

    // Bindings for cmd creation
    let file_path = file.path().to_string_lossy();
    let rpc_url = validator.rpc_url();
    let rpc_pubsub_url = validator.rpc_pubsub_url();

    // action
    let args = vec![
        "xtask",
        "solana",
        "deploy",
        "-k",
        &file_path,
        "-u",
        &rpc_url,
        "-w",
        &rpc_pubsub_url,
        "-p",
        file_program_id.path().to_str().unwrap(),
        "gmp-gateway",
    ];
    let cli: Cli = Cli::try_parse_from(args).unwrap();
    cli.run().await.unwrap();

    // assert
    dbg!(&keypair.pubkey());
    let validator_rpc_client = validator.get_async_rpc_client();
    let account_info = validator_rpc_client
        .get_account(&program_id.pubkey())
        .await
        .unwrap();
    assert!(account_info.executable);
}

#[ignore]
#[tokio::test]
#[serial]
async fn initialize_gateway_contract_works() {
    // Setup
    solana_logger::setup_with_default("solana_program_runtime=warn");
    build_gateway_contract();
    // Bring up the validator + the target contract to initialise.
    let mut seed_validator = TestValidatorGenesis::default();
    let program_id = gmp_gateway::id();
    seed_validator.add_upgradeable_programs_with_path(&[UpgradeableProgramInfo {
        program_id,
        loader: solana_sdk::bpf_loader_upgradeable::id(),
        upgrade_authority: program_id,
        program_path: cmd::solana::path::contracts_artifact_dir()
            .join(SolanaContract::GmpGateway.file()),
    }]);
    let (validator, keypair) = seed_validator.start_async().await;
    // Save private keypair to temp file for the test
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{:?}", keypair.to_bytes()).unwrap();
    // Prepare cmd
    let rpc_url = validator.rpc_url();
    let payer_kp = file.path().to_string_lossy();
    let args = vec![
        "xtask",
        "solana",
        "init",
        "gmp-gateway",
        "--rpc-url",
        &rpc_url,
        "--payer-kp-path",
        &payer_kp, // We use the already funded keypair.
        "--auth-weighted-file",
        "tests/gateway_init_config.toml",
    ];
    // Wait to programs to be consolidated in the validator.
    thread::sleep(Duration::from_millis(15000));

    // Execute CLI
    let cli: Cli = Cli::try_parse_from(args).unwrap();
    cli.run().await.unwrap();

    // Assert
    let validator_rpc_client = validator.get_async_rpc_client();
    let accounts = validator_rpc_client
        .get_program_accounts(&program_id)
        .await
        .unwrap();
    let account = accounts.first().unwrap().clone().1;
    assert_eq!(account.owner, gmp_gateway::id());

    // Expected values from the tests/auth_weighted.toml file

    let mut threshold = 0;
    let signers: BTreeMap<PublicKey, axelar_rkyv_encoding::types::U256> = [
        (
            "092c3da15c17a1e3eb01ed279684cc197a9938bde2dc1e59835a61afa6fb17ad64",
            1u128,
        ),
        (
            "508efe1eb50545edd0f762ba61290c579d513a38239bebaa97379628cefe82e62d",
            2,
        ),
    ]
    .into_iter()
    .map(|(ecdsa_pubkey_hex, weight)| {
        threshold += weight;
        let pubkey_bytes = hex::decode(ecdsa_pubkey_hex).unwrap();
        let public_key = PublicKey::Ecdsa(pubkey_bytes.try_into().unwrap());
        (public_key, weight.into())
    })
    .collect();

    let verifier_set = VerifierSet::new(0, signers, threshold.into());

    let hardcoded_operator =
        solana_sdk::pubkey::Pubkey::from_str("3KS2k14CmtnuVv2fvYcvdrNgC94Y11WETBpMUGgXyWZL")
            .unwrap();

    let auth_weighted = AxelarAuthWeighted::new(verifier_set);
    let (_, bump) = GatewayConfig::pda();

    let domain_separator = [0u8; 32]; // FIXME: fetch the domain separator from somewhere
    let gateway_config =
        GatewayConfig::new(bump, auth_weighted, hardcoded_operator, domain_separator);
    let deserialized_gateway_config = from_slice::<GatewayConfig>(&account.data).unwrap();
    assert_eq!(deserialized_gateway_config, gateway_config);
}
