use std::str::FromStr;
use std::{collections::HashMap, path::PathBuf};

use clap::{Parser, Subcommand};
use eyre::OptionExt;
use itertools::Itertools;
use xshell::{cmd, Shell};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Test {
        /// Will test contracts by default using sbf-test.
        /// This flag will ensure that we also run non-sbf tests
        #[clap(short, long, default_value_t = false)]
        only_sbf: bool,
    },
    Build {
        /// Network environment: devnet-amplifier, stagenet, testnet, or mainnet
        #[clap(short, long)]
        network: Option<String>,
    },
    Check,
    Fmt,
    UnusedDeps,
    Typos,
    Docs,
    CreateBindings {
        program: String,
        /// Copies them from temp folder to corresponding
        #[clap(short, long, default_value_t = false)]
        update: bool,
    },
    Audit {
        #[clap(last = true)]
        args: Vec<String>,
    },
    Deny {
        #[clap(last = true)]
        args: Vec<String>,
    },
    UpdateIds,
}

fn main() -> eyre::Result<()> {
    color_eyre::install()?;
    let sh = Shell::new()?;
    let args = Args::parse();

    match args.command {
        Commands::Test { only_sbf } => {
            println!("cargo test");
            let (solana_programs, auxiliary_crates) = workspace_crates_by_category(&sh)?;

            // build all solana programs (because they have internal inter-dependencies)
            for (_program, path) in solana_programs.iter() {
                let manifest_path = path.join("Cargo.toml");
                cmd!(sh, "cargo build-sbf --manifest-path {manifest_path}").run()?;
            }

            // test solana programs using `test-sbf`
            for (program, ..) in solana_programs.iter() {
                cmd!(sh, "cargo test-sbf -p {program}").run()?;
            }
            if only_sbf {
                return Ok(());
            }
            // test the other crates
            for (normal_crate, ..) in auxiliary_crates {
                cmd!(sh, "cargo test -p {normal_crate}").run()?;
            }
        }
        Commands::Build { network } => {
            println!("cargo build");

            // Validate network parameter if provided
            let valid_networks = ["devnet-amplifier", "stagenet", "testnet", "mainnet"];
            if let Some(ref net) = network {
                if !valid_networks.contains(&net.as_str()) {
                    return Err(eyre::eyre!(
                        "Invalid network '{}'. Must be one of: devnet-amplifier, stagenet, testnet, mainnet",
                        net
                    ));
                }
            }

            let (solana_programs, _auxiliary_crates) = workspace_crates_by_category(&sh)?;

            // build all solana programs (because they have internal inter-dependencies)
            for (_program, path) in solana_programs.iter() {
                let manifest_path = path.join("Cargo.toml");

                if let Some(ref net) = network {
                    println!("Building with network feature: {}", net);
                    cmd!(
                        sh,
                        "cargo build-sbf --manifest-path {manifest_path} --features {net} --no-default-features"
                    )
                    .run()?;
                } else {
                    cmd!(sh, "cargo build-sbf --manifest-path {manifest_path}").run()?;
                }
            }
        }
        Commands::Check => {
            println!("cargo check");
            cmd!(
                sh,
                "cargo clippy --no-deps --all-targets --workspace --locked -- -D warnings"
            )
            .run()?;
            cmd!(sh, "cargo fmt --all --check").run()?;
        }
        Commands::Fmt => {
            println!("cargo fix");
            cmd!(sh, "cargo fmt --all").run()?;
            cmd!(
                sh,
                "cargo fix --allow-dirty --allow-staged --workspace --tests"
            )
            .run()?;
            cmd!(
                sh,
                "cargo clippy --fix --allow-dirty --allow-staged --workspace --tests"
            )
            .run()?;
        }
        Commands::UnusedDeps => {
            println!("unused deps");
            cmd!(sh, "cargo +nightly install cargo-machete").run()?;
            cmd!(sh, "cargo-machete").run()?;
        }
        Commands::Typos => {
            println!("typos check");
            cmd!(sh, "cargo install typos-cli").run()?;
            cmd!(sh, "typos").run()?;
        }
        Commands::Docs => {
            println!("cargo doc");
            cmd!(sh, "cargo doc --workspace --no-deps").run()?;

            if std::option_env!("CI").is_none() {
                #[cfg(target_os = "macos")]
                cmd!(sh, "open target/doc/relayer/index.html").run()?;

                #[cfg(target_os = "linux")]
                cmd!(sh, "xdg-open target/doc/relayer/index.html").run()?;
            }
        }
        Commands::CreateBindings { program, update } => {
            println!("Creating bindings for: {}", program);
            let program = "axelar-solana-".to_owned() + &program;
            let temp_folder = "bindings/generated/temp/".to_owned();
            let temp_folder_program = temp_folder.clone() + &program;

            if std::fs::metadata(&temp_folder).is_err() {
                cmd!(sh, "mkdir {temp_folder}").run()?;
            }
            if std::fs::metadata(&temp_folder_program).is_err() {
                cmd!(sh, "mkdir {temp_folder_program}").run()?;
            }
            cmd!(
                sh,
                "../native-to-anchor/generator/target/debug/native-to-anchor package
                programs/{program}
                -o bindings/generated/temp
                -d bindings/anchor_lib/{program}.rs
                -k"
            )
            .run()?;
            if update {
                cmd!(
                    sh,
                    "rm -rf bindings/generated/{program}/src bindings/generated/{program}/idl.json"
                )
                .run()?;
                cmd!(
                    sh,
                    "cp -r bindings/generated/temp/{program}/src bindings/generated/{program}/"
                )
                .run()?;
                cmd!(
                    sh,
                    "cp bindings/generated/temp/{program}/idl.json bindings/generated/{program}/"
                )
                .run()?;
            }
        }
        Commands::Audit { args } => {
            println!("cargo audit");
            cmd!(sh, "cargo install cargo-audit --locked").run()?;
            cmd!(sh, "cargo audit {args...}").run()?;
        }
        Commands::Deny { args } => {
            println!("cargo deny");
            cmd!(sh, "cargo +nightly install cargo-deny").run()?;
            cmd!(sh, "cargo deny check {args...}").run()?;
        }
        Commands::UpdateIds => {
            println!("Updating program IDs");
            let program_prefixes = [
                ("axelar-solana-gateway", "gtw"),
                ("axelar-solana-its", "its"),
                ("axelar-solana-gas-service", "gas"),
                ("axelar-solana-multicall", "mc"),
                ("axelar-solana-memo-program", "mem"),
                ("axelar-solana-governance", "gov"),
            ];

            let (solana_programs, _) = workspace_crates_by_category(&sh)?;

            for (program_name, program_path) in solana_programs {
                if let Some((_, prefix)) = program_prefixes
                    .iter()
                    .find(|(name, _)| program_name == *name)
                {
                    println!("Regenerating ID for {program_name} with prefix {prefix}");
                    let lib_rs_path = program_path.join("src/lib.rs");

                    if !lib_rs_path.exists() {
                        println!("Warning: {lib_rs_path:?} not found, skipping");
                        continue;
                    }

                    // Generate new program ID using solana-keygen grind
                    let output =
                        cmd!(sh, "solana-keygen grind --starts-with {prefix}:1").output()?;

                    // Parse the output to extract the pubkey
                    let output_str = String::from_utf8(output.stdout)?;
                    let mut new_id = String::new();
                    for line in output_str.lines() {
                        if line.contains(".json") {
                            if let Some(filename) = line.split_whitespace().last() {
                                if let Some(pubkey) = filename.split('.').next() {
                                    new_id = pubkey.to_string();
                                    break;
                                }
                            }
                        }
                    }

                    if new_id.is_empty() {
                        println!("Failed to generate new ID for {program_name}");
                        continue;
                    }

                    println!("Generated new ID for {program_name}: {new_id}");

                    // Update the declare_id! macro in lib.rs
                    // Read the current lib.rs content
                    let lib_content = std::fs::read_to_string(&lib_rs_path)?;
                    let updated_content = lib_content.replace(
                        lib_content
                            .lines()
                            .find(|line| line.contains("solana_program::declare_id!("))
                            .unwrap_or("declare_id!(\"NoMatch\");"),
                        &format!("solana_program::declare_id!(\"{}\");", new_id),
                    );

                    std::fs::write(&lib_rs_path, updated_content)?;
                    println!("Updated declare_id! macro in {lib_rs_path:?}");
                }
            }

            println!("Program IDs regenerated and successfully updated");
        }
    }

    Ok(())
}

type WorkspaceCrateInfo<'a> = (&'a str, PathBuf);

/// Return all crates in the workspace sorted by category:
/// - (solana program crates, native crates)
fn workspace_crates_by_category(
    sh: &Shell,
) -> Result<(Vec<WorkspaceCrateInfo>, Vec<WorkspaceCrateInfo>), eyre::Error> {
    let crates_in_repo = cmd!(sh, "cargo tree --workspace --depth 0")
        .output()
        .map(|o| String::from_utf8(o.stdout))??
        .leak(); // fine to leak as xtask is short lived
    let all_crate_data = crates_in_repo.split_whitespace();
    let all_crate_data = all_crate_data
        .filter(|item| !item.starts_with('[') && !item.contains("proc-macro")) // filters "[dev-dependencies]" and "(proc-macro)"
        .tuples()
        .group_by(|(_, _, path)| path.contains("programs"));
    let mut solana_programs = HashMap::new();
    let mut auxiliary_crates = HashMap::new();
    for (is_solana_program, group) in &all_crate_data {
        for (crate_name, _crate_version, crate_path) in group {
            let crate_path = crate_path
                .strip_prefix('(')
                .ok_or_eyre("expected prefix not there")?;
            let crate_path = crate_path
                .strip_suffix(')')
                .ok_or_eyre("expected suffix not there")?;
            let crate_path = PathBuf::from_str(crate_path)?;
            if is_solana_program {
                solana_programs.insert(crate_name, crate_path);
            } else {
                auxiliary_crates.insert(crate_name, crate_path);
            }
        }
    }
    Ok((
        solana_programs.into_iter().collect(),
        auxiliary_crates.into_iter().collect(),
    ))
}
