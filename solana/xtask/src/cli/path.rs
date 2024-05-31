use std::env;
use std::path::{Path, PathBuf};

pub fn workspace_root_dir() -> anyhow::Result<PathBuf> {
    let dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    Ok(PathBuf::from(dir).parent().unwrap().to_owned())
}

pub fn contracts_dir() -> anyhow::Result<PathBuf> {
    Ok(workspace_root_dir()?.join("programs"))
}

pub fn contracts_artifact_dir() -> anyhow::Result<PathBuf> {
    Ok(workspace_root_dir()?.join("target").join("deploy"))
}

pub fn ensure_optional_path_exists(path: Option<&PathBuf>, subject: &str) -> anyhow::Result<()> {
    match path {
        Some(path) => ensure_path_exists(path, subject),
        None => Ok(()),
    }
}

pub fn ensure_path_exists(path: &Path, subject: &str) -> anyhow::Result<()> {
    match path.exists() {
        true => Ok(()),
        false => Err(anyhow::anyhow!(
            "File {} do not exists or it's not readable at: {}",
            subject.to_lowercase(),
            path.to_string_lossy()
        )),
    }
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
        assert!(result.is_err())
    }

    #[test]
    fn ensure_optional_path_makes_a_positive_when_none() {
        ensure_optional_path_exists(None, "A non required file").unwrap();
    }
}
