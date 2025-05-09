use std::path::PathBuf;
use std::{fs, str::FromStr};

use crate::types::ChainsInfoFile;
use crate::{
    error::{AppError, Result},
    types::NetworkType,
};

#[derive(Debug)]
pub struct Config {
    pub url: String,
    pub output_dir: PathBuf,
    pub network_type: NetworkType,
    pub chains_info_file: PathBuf,
}

impl Config {
    pub fn new(url: String, output_dir: PathBuf, chains_info_dir: PathBuf) -> Result<Self> {
        println!("URL: {}", url);
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir).map_err(AppError::IoError)?;
            println!("Created output directory: {}", output_dir.display());
        } else if !output_dir.is_dir() {
            return Err(AppError::ConfigError(format!(
                "Specified output path exists but is not a directory: {}",
                output_dir.display()
            )));
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
