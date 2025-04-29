use std::path::PathBuf;
use std::{fs, str::FromStr};

use crate::{
    error::{AppError, Result},
    types::NetworkType,
};

#[derive(Debug, Clone)]
pub struct Config {
    pub url: String,
    pub output_dir: PathBuf,
    pub network_type: NetworkType,
}

impl Config {
    pub fn new(url: String, output_dir: PathBuf) -> Result<Self> {
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir).map_err(|e| AppError::IoError(e))?;
            println!("Created output directory: {}", output_dir.display());
        } else if !output_dir.is_dir() {
            return Err(AppError::ConfigError(format!(
                "Specified output path exists but is not a directory: {}",
                output_dir.display()
            )));
        }

        let network_type = NetworkType::from_str(&url)?;

        Ok(Self {
            url,
            output_dir,
            network_type,
        })
    }
}
