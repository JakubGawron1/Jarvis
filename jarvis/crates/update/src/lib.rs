use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub core_version: String,
    pub git: String,
    #[serde(default)]
    pub artifacts: serde_json::Map<String, serde_json::Value>,
}

pub fn load_manifest(path: &Path) -> Result<Manifest> {
    let s = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&s)?)
}

pub fn sha256_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

/// Compare local version to manifest. Returns Some(new_version) if an update exists.
pub fn needs_pull(local: &str, remote: &Manifest) -> Option<String> {
    if remote.core_version != local && remote.core_version != "0.0.0" {
        Some(remote.core_version.clone())
    } else {
        None
    }
}

pub async fn download_artifact(url: &str, dest: &Path) -> Result<()> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, bytes)?;
    Ok(())
}

/// Desktop rewrite helper: run `cargo test` in a worktree. Caller owns git.
pub fn cargo_test(dir: &Path) -> Result<bool> {
    let st = std::process::Command::new("cargo")
        .arg("test")
        .arg("--workspace")
        .current_dir(dir)
        .status()?;
    Ok(st.success())
}
