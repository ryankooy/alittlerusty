use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub subdirs: Vec<String>,
    pub hidden_files: Option<Vec<String>>,
    pub drives: Vec<Drive>,
    pub gd_folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Drive {
    pub mountpoint: String,
    pub drive: String,
    pub dir: String,
    pub desc: String,
}

/// Read and parse config values from toml file.
pub fn get_config() -> Result<Config> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "cfg.toml"]
        .iter()
        .collect();

    let cfg_str = fs::read_to_string(&path)
        .with_context(|| {
            format!("Failed to read config file {}", path.display())
        })?;

    let config: Config = toml::from_str(&cfg_str)
        .context("Failed to parse config")?;

    Ok(config)
}
