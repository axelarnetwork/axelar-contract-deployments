#[allow(deprecated)]
use std::env::home_dir;
use std::path::PathBuf;
use std::str::FromStr;

use solana_cli_config::Config;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::EncodableKey;
use url::Url;

use crate::cli::path::ensure_path_exists;

/// If provided, it parses the Keypair from the provided
/// path. If not provided, it calculates and uses default Solana CLI
/// keypair path. Finally, it tries to read the file.
pub fn payer_kp_with_fallback_in_sol_cli_config(
    payer_kp_path: &Option<PathBuf>,
) -> anyhow::Result<Keypair> {
    let calculated_payer_kp_path = match payer_kp_path {
        Some(kp_path) => kp_path.clone(),
        None => PathBuf::from(Config::default().keypair_path),
    };
    ensure_path_exists(&calculated_payer_kp_path, "payer keypair")?;
    Keypair::read_from_file(&calculated_payer_kp_path)
        .map_err(|_| anyhow::Error::msg("Could not read payer key pair"))
}

/// If provided, it parses the provided RPC URL. If not provided,
/// it calculates and uses default Solana CLI
/// rpc URL.
pub fn rpc_url_with_fallback_in_sol_cli_config(rpc_url: &Option<Url>) -> anyhow::Result<Url> {
    let calculated_rpc_url = match rpc_url {
        Some(kp_path) => kp_path.clone(),
        None => {
            #[allow(deprecated)]
            // We are not explicitly supporting windows, plus home_dir() is what solana is using
            // under the hood.
            let mut sol_config_path = home_dir().ok_or(anyhow::anyhow!("Home dir not found !"))?;
            sol_config_path.extend([".config", "solana", "cli", "config.yml"]);

            let sol_cli_config = Config::load(
                sol_config_path
                    .to_str()
                    .ok_or(anyhow::anyhow!("Config path not valid unicode !"))?,
            )?;
            Url::from_str(&sol_cli_config.json_rpc_url)?
        }
    };

    Ok(calculated_rpc_url)
}
