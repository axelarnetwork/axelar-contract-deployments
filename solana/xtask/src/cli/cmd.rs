pub(crate) mod cosmwasm;
pub(crate) mod evm;
pub(crate) mod solana;
pub(crate) mod testnet;

pub(crate) mod path {
    use std::path::{Path, PathBuf};

    /// Return the [`PathBuf`] that points to the `[repo]/solana` folder
    pub(crate) fn workspace_root_dir() -> PathBuf {
        let dir = std::env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
        PathBuf::from(dir).parent().unwrap().to_owned()
    }

    pub(crate) fn ensure_optional_path_exists(
        path: Option<&PathBuf>,
        subject: &str,
    ) -> eyre::Result<()> {
        match path {
            Some(path) => ensure_path_exists(path, subject),
            None => Ok(()),
        }
    }

    pub(crate) fn ensure_path_exists(path: &Path, subject: &str) -> eyre::Result<()> {
        path.exists().then(|| Ok(())).unwrap_or_else(|| {
            Err(eyre::eyre!(
                "File {} do not exists or it's not readable at: {}",
                subject.to_lowercase(),
                path.to_string_lossy()
            ))
        })
    }

    #[cfg(test)]
    mod tests {
        use tempfile::NamedTempFile;

        use super::*;

        #[test]
        fn ensure_optional_path_makes_a_positive() {
            let tempfile = NamedTempFile::new().unwrap();
            ensure_optional_path_exists(
                Some(&tempfile.path().to_path_buf()),
                "A required file on fs",
            )
            .unwrap();
        }

        #[test]
        fn ensure_optional_path_makes_a_negative() {
            let temp_file = NamedTempFile::new().unwrap();
            let path = &temp_file.path().to_path_buf();
            drop(temp_file);
            let result = ensure_optional_path_exists(Some(path), "A required file on fs");
            assert!(result.is_err());
        }

        #[test]
        fn ensure_optional_path_makes_a_positive_when_none() {
            ensure_optional_path_exists(None, "A non required file").unwrap();
        }
    }
}
