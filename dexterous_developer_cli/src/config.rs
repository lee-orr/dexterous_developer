use std::collections::HashMap;

use dexterous_developer_types::Target;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::info;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct DexterousConfig {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub targets: HashMap<Target, ReloadTargetConfig>,
    #[serde(default)]
    pub packages: HashMap<String, ReloadTargetConfig>,
    #[serde(default)]
    pub examples: HashMap<String, ReloadTargetConfig>,
    #[serde(default)]
    pub default_package: Option<ReloadTargetConfig>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ReloadTargetConfig {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
}

impl DexterousConfig {
    pub async fn load_toml(path: &camino::Utf8Path) -> Result<Self, LoadConfigError> {
        let path = path.canonicalize_utf8()?;
        let path = if path.is_file() {
            path
        } else {
            path.join("Dexterous.toml")
        };

        if !path.exists() {
            info!("No config found at {path}, using a default config");
            return Ok(Default::default());
        }

        let file = tokio::fs::read_to_string(path).await?;

        let config = toml::from_str(&file)?;

        Ok(config)
    }

    pub fn load_toml_from_str(toml: &str) -> Result<Self, LoadConfigError> {
        let config = toml::from_str(toml)?;
        Ok(config)
    }
}

#[derive(Error, Debug)]
pub enum LoadConfigError {
    #[error("Couldn't read config file {0}")]
    IoError(#[from] std::io::Error),
    #[error("Couldn't parse config file {0}")]
    ParseError(#[from] toml::de::Error),
}
