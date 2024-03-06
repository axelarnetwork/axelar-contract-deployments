use anyhow::{bail, ensure};
use clap::Parser;
use serde::{Deserialize, Deserializer};
use solana_program::pubkey::Pubkey;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use url::Url;

/// Solana network name used to identify it on Axelar.
pub const SOLANA_CHAIN_NAME: &str = "solana";

/// Solana GMP Gateway root config PDA

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the configuration file
    #[arg(long)]
    pub config: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub axelar_to_solana: Option<AxelarToSolana>,
    pub solana_to_axelar: Option<SolanaToAxelar>,
    pub database: Database,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        let directions = (&self.axelar_to_solana, &self.solana_to_axelar);

        if let (None, None) = directions {
            bail!("Relayer must be configured with at least one message transport direction")
        }

        if let Some(axelar_to_solana) = &self.axelar_to_solana {
            axelar_to_solana.validate()?
        }
        if let Some(_solana_to_axelar) = &self.solana_to_axelar {
            // Put relevant validation logic here
        }

        Ok(())
    }
}

#[derive(Deserialize, Debug)]
pub struct AxelarToSolana {
    pub approver: AxelarApprover,
    pub includer: SolanaIncluder,
}
impl AxelarToSolana {
    fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            self.includer.keypair_file.exists(),
            "Solana keypair file does not exist"
        );
        Ok(())
    }
}

#[derive(Deserialize, Debug)]
pub struct SolanaToAxelar {
    pub sentinel: SolanaSentinel,
    pub verifier: AxelarVerifier,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub url: Url,
}

#[derive(Deserialize, Debug)]
pub struct AxelarApprover {
    pub rpc: Url,
}

#[derive(Deserialize, Debug)]
pub struct SolanaIncluder {
    pub rpc: Url,
    pub keypair_file: PathBuf,
}

#[derive(Deserialize, Debug)]
pub struct SolanaSentinel {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub gateway_address: Pubkey,
    pub rpc: Url,
}

#[derive(Deserialize, Debug)]
pub struct AxelarVerifier {
    pub rpc: Url,
}

pub fn parse_command_line_args() -> anyhow::Result<Config> {
    let args = Args::parse();
    let config: Config = toml::from_str(&read_to_string(args.config)?)?;
    config.validate()?;
    Ok(config)
}

fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_full_config() {
        let toml_str = r#"
            [axelar_to_solana.approver]
            rpc = "https://approver.axelar.rpc.url"

            [axelar_to_solana.includer]
            rpc = "https://includer.solana.rpc.url"
            keypair_file = "/path/to/solana/keypair/file"

            [solana_to_axelar.sentinel]
            gateway_address = "5ScCroHMfw56UbnLPAYxM61WSumAwS7hDwymNvkWfA5E"
            rpc = "https://sentinel.solana.rpc.url"

            [solana_to_axelar.verifier]
            rpc = "https://verifier.axelar.rpc.url"

            [database]
            url = "postgres://username:password@localhost:5432/database_name"
        "#;

        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn parse_partial_inbound_config() {
        let toml_str = r#"
            [axelar_to_solana.approver]
            rpc = "https://approver.axelar.rpc.url"

            [axelar_to_solana.includer]
            rpc = "https://includer.solana.rpc.url"
            keypair_file = "/path/to/solana/keypair/file"

            [database]
            url = "postgres://username:password@localhost:5432/database_name"
        "#;

        toml::from_str::<Config>(toml_str).unwrap();
    }

    #[test]
    fn parse_partial_outbound_config() {
        let toml_str = r#"
            [solana_to_axelar.sentinel]
            keypair_file = "/path/to/solana/keypair/file"
            gateway_address = "5ScCroHMfw56UbnLPAYxM61WSumAwS7hDwymNvkWfA5E"
            rpc = "https://sentinel.solana.rpc.url"

            [solana_to_axelar.verifier]
            rpc = "https://verifier.axelar.rpc.url"

            [database]
            url = "postgres://username:password@localhost:5432/database_name"
        "#;

        toml::from_str::<Config>(toml_str).unwrap();
    }
}
