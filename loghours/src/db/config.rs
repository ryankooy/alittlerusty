use std::path::PathBuf;
use anyhow::{bail, Context, Result};
use toml::Table;

pub struct Config {
    pub path: Option<PathBuf>,
}

impl Config {
    pub fn new() -> Self {
        Self { path: None }
    }

    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    pub fn get_path(&self) -> Option<PathBuf> {
        self.path.clone()
    }
}

pub fn get_config() -> Result<Config> {
    let mut config = Config::new();
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "db.toml"].iter().collect();

    let cfg_table = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config file {}", path.display()))?
        .parse::<Table>()
        .context("Failed to parse config")?;

    match cfg_table.get("path") {
        None => bail!("No db path configured"),
        Some(db_path) => {
            if let Some(path) = db_path.as_str() {
                config.set_path(PathBuf::from(path));
            }
        }
    };

    Ok(config)
}
