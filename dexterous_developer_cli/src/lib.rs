use std::collections::HashMap;

use cargo_toml::Manifest;
use dexterous_developer_types::Target;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ReloadMetadata {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub targets: HashMap<Target, ReloadTargetMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CargoMetadata {
    #[serde(default)]
    reload: Option<ReloadMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ReloadTargetMetadata {
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub asset_folders: Vec<camino::Utf8PathBuf>,
}

pub fn load_cargo_toml<Metadata: ReloadMetadataContainer>(
    working_directory: &camino::Utf8Path,
) -> Result<Manifest<Metadata>, cargo_toml::Error> {
    let manifest = cargo_toml::Manifest::<Metadata>::from_path_with_metadata(working_directory)?;
    Ok(manifest)
}

pub fn load_cargo_toml_from_str<Metadata: ReloadMetadataContainer>(
    toml: &str,
) -> Result<Manifest<Metadata>, cargo_toml::Error> {
    let manifest = cargo_toml::Manifest::<Metadata>::from_slice_with_metadata(toml.as_bytes())?;
    Ok(manifest)
}

pub trait ReloadMetadataContainer: Serialize + DeserializeOwned {
    fn get_reload_metadata(&self) -> Option<&ReloadMetadata>;
}

impl ReloadMetadataContainer for CargoMetadata {
    fn get_reload_metadata(&self) -> Option<&ReloadMetadata> {
        self.reload.as_ref()
    }
}
