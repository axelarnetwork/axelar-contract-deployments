use std::path::PathBuf;
use std::str::FromStr;

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
    Build,
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
        Commands::Build => {
            println!("cargo build");
            let (solana_programs, _auxiliary_crates) = workspace_crates_by_category(&sh)?;

            // build all solana programs (because they have internal inter-dependencies)
            for (_program, path) in solana_programs.iter() {
                let manifest_path = path.join("Cargo.toml");
                cmd!(sh, "cargo build-sbf --manifest-path {manifest_path}").run()?;
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
                "cargo fix --allow-dirty --allow-staged --workspace --all-features --tests"
            )
            .run()?;
            cmd!(
                sh,
                "cargo clippy --fix --allow-dirty --allow-staged --workspace --all-features --tests"
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
            cmd!(sh, "cargo doc --workspace --no-deps --all-features").run()?;

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
                "../../native-to-anchor/generator/target/debug/native-to-anchor package
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
        .filter(|item| !item.starts_with('[')) // filters "[dev-dependencies]"
        .tuples()
        .group_by(|(_, _, path)| path.contains("solana/programs"));
    let mut solana_programs = vec![];
    let mut auxiliary_crates = vec![];
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
                solana_programs.push((crate_name, crate_path))
            } else {
                auxiliary_crates.push((crate_name, crate_path))
            }
        }
    }
    Ok((solana_programs, auxiliary_crates))
}
