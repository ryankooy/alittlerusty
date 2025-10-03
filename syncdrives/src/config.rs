use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub subdirs: Vec<String>,
    pub hidden_files: Option<Vec<String>>,
    pub drives: Vec<Drive>,
    pub gd_folder_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Drive {
    /// Drive letter
    #[serde(deserialize_with = "deserialize_drive_letter")]
    pub letter: String,

    /// Drive's nickname
    pub nickname: Option<String>,

    /// Custom base directory
    pub base_dir: Option<String>,
}

fn deserialize_drive_letter<'a, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'a>
{
    let letter = String::deserialize(deserializer)?;
    Ok(deformat_drive_letter(&letter))
}

impl Drive {
    pub fn new(
        letter: String,
        nickname: Option<String>,
        base_dir: Option<String>,
    ) -> Self {
        Self {
            letter: deformat_drive_letter(&letter),
            nickname,
            base_dir,
        }
    }

    pub fn get_letter(&self) -> String {
        format_drive_letter(&self.letter)
    }

    pub fn get_nickname(&self) -> String {
        if let Some(name) = &self.nickname {
            name.to_string()
        } else {
            "External Drive".to_string()
        }
    }

    pub fn get_base_dir(&self) -> String {
        if let Some(dir) = &self.base_dir {
            format!("/mnt/{}/{}", self.letter, dir.trim_end_matches('/'))
        } else {
            self.get_mountpoint()
        }
    }

    pub fn get_mountpoint(&self) -> String {
        format!("/mnt/{}", self.letter)
    }
}

/// Read and parse config values from toml file.
pub fn get_config() -> Result<Config> {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "config.toml"]
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

fn format_drive_letter(letter: &String) -> String {
    format!("{}:", letter.to_uppercase().trim_end_matches(':'))
}

fn deformat_drive_letter(letter: &String) -> String {
    letter.to_lowercase().trim_end_matches(':').to_string()
}
