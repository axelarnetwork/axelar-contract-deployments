use eyre::eyre;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use crate::types::ChainsInfoFile;
use crate::types::NetworkType;

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) url: String,
    pub(crate) output_dir: PathBuf,
    pub(crate) network_type: NetworkType,
    pub(crate) chains_info_file: PathBuf,
}

impl Config {
    pub(crate) fn new(
        url: String,
        output_dir: PathBuf,
        chains_info_dir: PathBuf,
    ) -> eyre::Result<Self> {
        println!("URL: {url}");
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir)
                .map_err(|e| eyre!("Failed to create output directory: {}", e))?;
            println!("Created output directory: {}", output_dir.display());
        }

        if !output_dir.is_dir() {
            eyre::bail!(
                "Specified output path exists but is not a directory: {}",
                output_dir.display()
            );
        }

        let network_type = NetworkType::from_str(&url)?;
        let chains_info_filename: String = ChainsInfoFile::from(network_type).into();
        let chains_info_file = chains_info_dir.join(chains_info_filename);

        Ok(Self {
            url,
            output_dir,
            network_type,
            chains_info_file,
        })
    }
}
