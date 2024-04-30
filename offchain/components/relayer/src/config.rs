use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::bail;
use clap::Parser;
use serde::{Deserialize, Deserializer};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use tonic::transport::Uri;
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

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Config {
    pub axelar_to_solana: Option<AxelarToSolana>,
    pub solana_to_axelar: Option<SolanaToAxelar>,
    pub database: Database,
    pub health_check: HealthCheck,
}

impl Config {
    pub fn from_file(path: &Path) -> anyhow::Result<Config> {
        let config_file = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&config_file)?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let (None, None) = (&self.axelar_to_solana, &self.solana_to_axelar) {
            bail!("Relayer must be configured with at least one message transport direction")
        }
        Ok(())
    }
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarToSolana {
    pub approver: AxelarApprover,
    pub includer: SolanaIncluder,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaToAxelar {
    pub sentinel: SolanaSentinel,
    pub verifier: AxelarVerifier,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Database {
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub url: Url,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct HealthCheck {
    #[serde(deserialize_with = "serde_utils::deserialize_socket_addr")]
    pub bind_addr: SocketAddr,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarApprover {
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaIncluder {
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
    #[serde(deserialize_with = "serde_utils::deserialize_keypair")]
    pub keypair: Arc<Keypair>,
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_address: Pubkey,
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_config_address: Pubkey,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaSentinel {
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_address: Pubkey,
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarVerifier {
    #[serde(deserialize_with = "serde_utils::deserialize_uri")]
    pub rpc: Uri,
}

mod serde_utils {
    use super::*;

    pub fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Arc<Keypair>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = from_env(deserializer)?;
        let bytes = solana_sdk::bs58::decode(s)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        Keypair::from_bytes(&bytes)
            .map(Arc::new)
            .map_err(serde::de::Error::custom)
    }

    pub fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = from_env(deserializer)?;
        Pubkey::from_str(&s).map_err(serde::de::Error::custom)
    }

    pub fn deserialize_socket_addr<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = from_env(deserializer)?;
        SocketAddr::from_str(&s).map_err(serde::de::Error::custom)
    }

    pub fn deserialize_uri<'de, D>(deserializer: D) -> Result<Uri, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = from_env(deserializer)?;
        Uri::from_str(&s).map_err(serde::de::Error::custom)
    }

    pub fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = from_env(deserializer)?;
        Url::from_str(&s).map_err(serde::de::Error::custom)
    }

    /// Deserializes a string and resolves it as an environment variable if
    /// prefixed with `$`.
    fn from_env<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        if let Some(env_var) = raw_string.strip_prefix('$') {
            std::env::var(env_var).map_err(serde::de::Error::custom)
        } else {
            Ok(raw_string)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_toml() -> anyhow::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[axelar_to_solana.approver]
            rpc = "http://0.0.0.1/"

            [axelar_to_solana.includer]
            rpc = "http://0.0.0.2/"
            keypair = "{relayer_keypair_b58}"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}"

            [solana_to_axelar.sentinel]
            gateway_address = "{gateway_address}"
            rpc = "http://0.0.0.3/"

            [solana_to_axelar.verifier]
            rpc = "http://0.0.0.4/"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#,
            relayer_keypair_b58 = relayer_keypair.to_base58_string()
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: Url::parse("http://0.0.0.1/")?,
                },
                includer: SolanaIncluder {
                    rpc: Url::parse("http://0.0.0.2/")?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    gateway_address,
                    rpc: Url::parse("http://0.0.0.3/")?,
                },
                verifier: AxelarVerifier {
                    rpc: Uri::from_static("http://0.0.0.4/"),
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
        };
        assert_eq!(parsed, expected);
        assert!(parsed.validate().is_ok());
        Ok(())
    }

    #[test]
    fn partial_axelar_to_solana() -> anyhow::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[axelar_to_solana.approver]
            rpc = "http://0.0.0.1/"

            [axelar_to_solana.includer]
            rpc = "http://0.0.0.2/"
            keypair = "{relayer_keypair_b58}"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#,
            relayer_keypair_b58 = relayer_keypair.to_base58_string()
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: Url::parse("http://0.0.0.1/")?,
                },
                includer: SolanaIncluder {
                    rpc: Url::parse("http://0.0.0.2/")?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: None,
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
        };
        assert_eq!(parsed, expected);
        assert!(parsed.validate().is_ok());
        Ok(())
    }

    #[test]
    fn partial_solana_to_axelar() -> anyhow::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[solana_to_axelar.sentinel]
            gateway_address = "{gateway_address}"
            rpc = "http://0.0.0.3/"

            [solana_to_axelar.verifier]
            rpc = "http://0.0.0.4/"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: None,
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    gateway_address,
                    rpc: Url::parse("http://0.0.0.3/")?,
                },
                verifier: AxelarVerifier {
                    rpc: Uri::from_static("http://0.0.0.4/"),
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
        };
        assert_eq!(parsed, expected);
        assert!(parsed.validate().is_ok());
        Ok(())
    }

    #[test]
    fn validation_fails_if_configured_without_transports() -> anyhow::Result<()> {
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";
        let input = format!(
            r#"[database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#
        );
        let parsed = toml::from_str::<Config>(&input)?;
        assert!(parsed.axelar_to_solana.is_none());
        assert!(parsed.solana_to_axelar.is_none());
        assert!(parsed.validate().is_err());
        Ok(())
    }

    #[test]
    fn deserialize_secret_key_from_env() -> anyhow::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let keypair = Keypair::new();
        let input = format!(
            r#"rpc = "http://0.0.0.1/"
            keypair = "$SECRET_KEY"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}""#
        );
        let parsed = temp_env::with_var("SECRET_KEY", Some(keypair.to_base58_string()), || {
            toml::from_str::<SolanaIncluder>(&input)
        })?;
        let expected = SolanaIncluder {
            rpc: Url::parse("http://0.0.0.1/")?,
            keypair: Arc::new(keypair),
            gateway_address,
            gateway_config_address,
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn deserialize_all_from_env() -> anyhow::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let approver_rpc = "http://0.0.0.1/";
        let includer_rpc = "http://0.0.0.2/";
        let sentinel_rpc = "http://0.0.0.3/";
        let verifier_rpc = "http://0.0.0.4/";

        let input = r#"[axelar_to_solana.approver]
                rpc = "$APPROVER_RPC"

                [axelar_to_solana.includer]
                rpc = "$INCLUDER_RPC"
                keypair = "$SECRET_KEY"
                gateway_address = "$GATEWAY_ADDRESS"
                gateway_config_address = "$GATEWAY_CONFIG_ADDRESS"

                [solana_to_axelar.sentinel]
                gateway_address = "$GATEWAY_ADDRESS"
                rpc = "$SENTINEL_RPC"

                [solana_to_axelar.verifier]
                rpc = "$VERIFIER_RPC"

                [database]
                url = "$DATABASE_URL"

                [health_check]
                bind_addr = "$HEALTHCHECK_BIND_ADDRESS""#;

        let parsed = temp_env::with_vars(
            [
                ("APPROVER_RPC", Some(approver_rpc)),
                ("INCLUDER_RPC", Some(includer_rpc)),
                ("SENTINEL_RPC", Some(sentinel_rpc)),
                ("VERIFIER_RPC", Some(verifier_rpc)),
                ("SECRET_KEY", Some(&relayer_keypair.to_base58_string())),
                ("GATEWAY_ADDRESS", Some(&gateway_address.to_string())),
                (
                    "GATEWAY_CONFIG_ADDRESS",
                    Some(&gateway_config_address.to_string()),
                ),
                ("DATABASE_URL", Some(database_url)),
                ("HEALTHCHECK_BIND_ADDRESS", Some(healthcheck_bind_addr)),
            ],
            || toml::from_str::<Config>(input),
        )?;

        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: Url::parse(approver_rpc)?,
                },
                includer: SolanaIncluder {
                    rpc: Url::parse(includer_rpc)?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    gateway_address,
                    rpc: Url::parse(sentinel_rpc)?,
                },
                verifier: AxelarVerifier {
                    rpc: Uri::from_static(verifier_rpc),
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
        };
        assert_eq!(parsed, expected);
        assert!(parsed.validate().is_ok());
        Ok(())
    }
}
