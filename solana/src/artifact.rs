use std::path::{Path, PathBuf};

use eyre::{Result, bail};
use regex::Regex;

use crate::types::Programs;

const AXELAR_R2_BASE_URL: &str = "https://static.axelar.network";
const GITHUB_RELEASES_BASE_URL: &str =
    "https://github.com/axelarnetwork/axelar-amplifier-solana/releases/download";

/// Get the download URL for a program artifact
/// - Semver (e.g., 0.1.7) → GitHub releases
/// - Commit hash (e.g., 12e6126) → R2
pub(crate) fn get_artifact_url(program: &Programs, version: &str) -> Result<String> {
    let package_name = program_to_package_name(program)?;
    let so_filename = program_to_so_filename(program);

    if is_semver(version) {
        Ok(format!(
            "{GITHUB_RELEASES_BASE_URL}/{package_name}-v{version}/{so_filename}.so"
        ))
    } else if is_commit_hash(version) {
        let version_lower = version.to_lowercase();
        Ok(format!(
            "{AXELAR_R2_BASE_URL}/releases/solana/{package_name}/{version_lower}/programs/{so_filename}.so"
        ))
    } else {
        bail!(
            "Invalid version '{}'. Use semver (e.g., 0.1.7) or commit hash (e.g., 12e6126)",
            version
        )
    }
}

/// Download a program artifact from GitHub releases or R2
pub(crate) async fn download_artifact(program: &Programs, version: &str) -> Result<PathBuf> {
    let url = get_artifact_url(program, version)?;
    let source = if is_semver(version) { "GitHub" } else { "R2" };
    println!(
        "Downloading {} from {} ({})",
        program_to_so_filename(program),
        source,
        url
    );

    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        bail!("Failed to download from {}: {}", url, response.status());
    }

    let bytes = response.bytes().await?;

    let artifacts_dir = PathBuf::from("./artifacts");
    std::fs::create_dir_all(&artifacts_dir)?;

    let normalized_version = if is_commit_hash(version) {
        version.to_lowercase()
    } else {
        version.to_owned()
    };
    let filename = format!(
        "{}-{}.so",
        program_to_so_filename(program),
        normalized_version
    );
    let path = artifacts_dir.join(&filename);
    std::fs::write(&path, &bytes)?;

    println!("Downloaded to {}", path.display());
    Ok(path)
}

/// Resolve .so path from a local artifact directory (e.g., target/deploy/)
pub(crate) fn resolve_from_artifact_dir(program: &Programs, dir: &Path) -> Result<PathBuf> {
    let filename = format!("{}.so", program_to_so_filename(program));
    let path = dir.join(&filename);
    if !path.exists() {
        bail!("Program binary not found at: {}", path.display());
    }
    Ok(path)
}

/// Resolve program path from one of three sources
pub(crate) async fn resolve_program_path(
    program: &Programs,
    program_path: &Option<String>,
    version: &Option<String>,
    artifact_dir: &Option<PathBuf>,
) -> Result<PathBuf> {
    match (program_path, version, artifact_dir) {
        (Some(path), None, None) => Ok(PathBuf::from(path)),
        (None, Some(ver), None) => download_artifact(program, ver).await,
        (None, None, Some(dir)) => resolve_from_artifact_dir(program, dir),
        (None, None, None) => {
            bail!("One of --program-path, --version, or --artifact-dir is required")
        }
        _ => unreachable!("clap ensures mutual exclusivity"),
    }
}

/// Check if string is a semantic version (e.g., 0.1.7)
fn is_semver(s: &str) -> bool {
    Regex::new(r"^\d+\.\d+\.\d+$")
        .expect("valid regex")
        .is_match(s)
}

fn is_commit_hash(s: &str) -> bool {
    Regex::new("(?i)^[a-f0-9]{7,}$")
        .expect("valid regex")
        .is_match(s)
}

/// Map program to package name (kebab-case)
fn program_to_package_name(program: &Programs) -> Result<&'static str> {
    match program {
        Programs::Gateway => Ok("solana-axelar-gateway"),
        Programs::GasService => Ok("solana-axelar-gas-service"),
        Programs::Governance => Ok("solana-axelar-governance"),
        Programs::Its => Ok("solana-axelar-its"),
        Programs::Operators => Ok("solana-axelar-operators"),
        Programs::Multicall => {
            bail!(
                "Multicall not available for download. Use --program-path or --artifact-dir instead."
            )
        }
    }
}

/// Map program to .so filename (snake_case, without extension)
pub(crate) fn program_to_so_filename(program: &Programs) -> &'static str {
    match program {
        Programs::Gateway => "solana_axelar_gateway",
        Programs::GasService => "solana_axelar_gas_service",
        Programs::Governance => "solana_axelar_governance",
        Programs::Its => "solana_axelar_its",
        Programs::Operators => "solana_axelar_operators",
        Programs::Multicall => "solana_axelar_multicall",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_semver() {
        assert!(is_semver("0.1.7"));
        assert!(is_semver("1.0.0"));
        assert!(is_semver("10.20.30"));
        assert!(!is_semver("v1.0.0"));
        assert!(!is_semver("1.0"));
        assert!(!is_semver("1.0.0.0"));
    }

    #[test]
    fn test_is_commit_hash() {
        assert!(is_commit_hash("12e6126"));
        assert!(is_commit_hash("abcdef1234567890"));
        assert!(is_commit_hash("12E6126"));
        assert!(is_commit_hash("ABCDEF1234567890"));
        assert!(is_commit_hash("AbCdEf1234567890"));
        assert!(!is_commit_hash("12e612"));
        assert!(!is_commit_hash("12e612g"));
    }

    #[test]
    fn test_get_artifact_url_semver() {
        let url = get_artifact_url(&Programs::Gateway, "0.1.7").unwrap();
        assert_eq!(
            url,
            "https://github.com/axelarnetwork/axelar-amplifier-solana/releases/download/solana-axelar-gateway-v0.1.7/solana_axelar_gateway.so"
        );

        let url = get_artifact_url(&Programs::Its, "1.0.0").unwrap();
        assert_eq!(
            url,
            "https://github.com/axelarnetwork/axelar-amplifier-solana/releases/download/solana-axelar-its-v1.0.0/solana_axelar_its.so"
        );
    }

    #[test]
    fn test_get_artifact_url_commit_hash() {
        let url = get_artifact_url(&Programs::Gateway, "12e6126").unwrap();
        assert_eq!(
            url,
            "https://static.axelar.network/releases/solana/solana-axelar-gateway/12e6126/programs/solana_axelar_gateway.so"
        );

        let url = get_artifact_url(&Programs::Its, "38e9135").unwrap();
        assert_eq!(
            url,
            "https://static.axelar.network/releases/solana/solana-axelar-its/38e9135/programs/solana_axelar_its.so"
        );
    }

    #[test]
    fn test_get_artifact_url_uppercase_commit_hash_normalized() {
        let url = get_artifact_url(&Programs::Gateway, "12E6126").unwrap();
        assert_eq!(
            url,
            "https://static.axelar.network/releases/solana/solana-axelar-gateway/12e6126/programs/solana_axelar_gateway.so"
        );

        let url = get_artifact_url(&Programs::Its, "ABCDEF1").unwrap();
        assert_eq!(
            url,
            "https://static.axelar.network/releases/solana/solana-axelar-its/abcdef1/programs/solana_axelar_its.so"
        );
    }

    #[test]
    fn test_invalid_version() {
        assert!(get_artifact_url(&Programs::Gateway, "invalid").is_err());
        assert!(get_artifact_url(&Programs::Gateway, "v1.0.0").is_err()); // no v prefix
    }

    #[test]
    fn test_multicall_not_available() {
        assert!(get_artifact_url(&Programs::Multicall, "0.1.7").is_err());
        assert!(get_artifact_url(&Programs::Multicall, "12e6126").is_err());
    }
}
