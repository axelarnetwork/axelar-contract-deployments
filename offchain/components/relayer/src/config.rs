use anyhow::{bail, Ok};
use clap::Parser;
use figment::{providers::Env, Figment};
use serde::{Deserialize, Deserializer};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
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

#[derive(Debug, Deserialize)]
pub struct ConfigEnv {
    pub database_url: Url,
    pub axelar_approver_url: Url,
    pub solana_includer_rpc: Url,
    #[serde(deserialize_with = "deserialize_keypair")]
    pub solana_includer_keypair: Keypair,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub sentinel_gateway_address: Pubkey,
    pub sentinel_rpc: Url,
    pub verifier_rpc: Url,
}

impl ConfigEnv {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new().merge(Env::prefixed("RELAYER_")).extract()
    }
}

#[derive(Deserialize, Debug, PartialEq)]
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

        if let Some(_axelar_to_solana) = &self.axelar_to_solana {}
        if let Some(_solana_to_axelar) = &self.solana_to_axelar {
            // Put relevant validation logic here
        }

        Ok(())
    }

    pub fn from_env() -> anyhow::Result<Config> {
        let config = ConfigEnv::load()?;
        Ok(Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: config.axelar_approver_url,
                },
                includer: SolanaIncluder {
                    rpc: config.solana_includer_rpc,
                    keypair: config.solana_includer_keypair,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    gateway_address: config.sentinel_gateway_address,
                    rpc: config.sentinel_rpc,
                },
                verifier: AxelarVerifier {
                    rpc: config.verifier_rpc,
                },
            }),
            database: Database {
                url: config.database_url,
            },
        })
    }
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct AxelarToSolana {
    pub approver: AxelarApprover,
    pub includer: SolanaIncluder,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct SolanaToAxelar {
    pub sentinel: SolanaSentinel,
    pub verifier: AxelarVerifier,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Database {
    pub url: Url,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct AxelarApprover {
    pub rpc: Url,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct SolanaIncluder {
    pub rpc: Url,
    #[serde(deserialize_with = "deserialize_keypair")]
    pub keypair: Keypair,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct SolanaSentinel {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub gateway_address: Pubkey,
    pub rpc: Url,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct AxelarVerifier {
    pub rpc: Url,
}

fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Keypair, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = solana_sdk::bs58::decode(s)
        .into_vec()
        .map_err(serde::de::Error::custom)?;
    Keypair::from_bytes(&bytes).map_err(serde::de::Error::custom)
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
    use std::env;

    use solana_sdk::signature::Keypair;

    use super::*;

    #[test]
    fn can_parse_config_from_env() {
        let db_url = "http://0.0.0.0/";
        let approver_url = "http://0.0.0.1/";
        let includer_rpc = "http://0.0.0.2/";
        let gw_addr = "4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a";
        let sentinel_rpc = "http://0.0.0.3/";
        let verifier_rpc = "http://0.0.0.4/";
        let keypair = Keypair::new();

        env::set_var("RELAYER_DATABASE_URL", db_url);
        env::set_var("RELAYER_AXELAR_APPROVER_URL", approver_url);
        env::set_var("RELAYER_SOLANA_INCLUDER_RPC", includer_rpc);
        env::set_var(
            "RELAYER_SOLANA_INCLUDER_KEYPAIR",
            keypair.to_base58_string(),
        );
        env::set_var("RELAYER_SENTINEL_GATEWAY_ADDRESS", gw_addr);
        env::set_var("RELAYER_SENTINEL_RPC", sentinel_rpc);
        env::set_var("RELAYER_VERIFIER_RPC", verifier_rpc);

        assert_eq!(
            Config::from_env().unwrap(),
            Config {
                axelar_to_solana: Some(AxelarToSolana {
                    approver: AxelarApprover {
                        rpc: Url::from_str(approver_url).unwrap()
                    },
                    includer: SolanaIncluder {
                        rpc: Url::from_str(includer_rpc).unwrap(),
                        keypair,
                    },
                }),
                solana_to_axelar: Some(SolanaToAxelar {
                    sentinel: SolanaSentinel {
                        gateway_address: Pubkey::from_str(gw_addr).unwrap(),
                        rpc: Url::from_str(sentinel_rpc).unwrap(),
                    },
                    verifier: AxelarVerifier {
                        rpc: Url::from_str(verifier_rpc).unwrap()
                    },
                }),
                database: Database {
                    url: Url::from_str(db_url).unwrap()
                },
            }
        );
    }
}
