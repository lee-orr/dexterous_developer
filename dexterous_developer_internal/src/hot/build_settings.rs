use std::path::PathBuf;

use anyhow::Error;

#[cfg(feature = "cli")]
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildSettings {
    pub watch_folder: PathBuf,
    pub manifest: Option<PathBuf>,
    pub lib_path: PathBuf,
    pub package: String,
    pub features: String,
    pub target_folder: Option<PathBuf>,
    pub out_target: PathBuf,
    pub build_target: Option<String>,
    pub updated_file_channel: Option<tokio::sync::broadcast::Sender<HotReloadMessage>>,
}

#[cfg(not(feature = "cli"))]
#[derive(Clone, Debug, Default)]
pub(crate) struct BuildSettings {
    pub watch_folder: PathBuf,
    pub manifest: Option<PathBuf>,
    pub lib_path: PathBuf,
    pub package: String,
    pub features: String,
    pub target_folder: Option<PathBuf>,
    pub out_target: PathBuf,
    pub build_target: Option<String>,
}

impl ToString for BuildSettings {
    fn to_string(&self) -> String {
        let BuildSettings {
            watch_folder,
            manifest,
            package,
            features,
            target_folder,
            lib_path,
            out_target,
            ..
        } = self;

        let target = target_folder
            .as_ref()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();

        let out_target = out_target.to_string_lossy().to_string();

        let watch_folder = watch_folder.to_string_lossy();
        let manifest = manifest
            .as_ref()
            .map(|v| v.to_string_lossy())
            .unwrap_or_default();
        let lib_path = lib_path.to_string_lossy();

        format!("{lib_path}:!:{watch_folder}:!:{manifest}:!:{package}:!:{features}:!:{out_target}:!:{target}")
    }
}

impl TryFrom<&str> for BuildSettings {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.split(":!:");
        let lib_path = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no library path"))?;
        let watch_folder = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no watch folder"))?;
        let manifest = split.next().filter(|v| !v.is_empty()).map(PathBuf::from);
        let package = split
            .next()
            .map(|v| v.to_string())
            .ok_or(Error::msg("no package"))?;
        let features = split
            .next()
            .map(|v| v.to_string())
            .ok_or(Error::msg("no features"))?;
        let out_target = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no out_target"))?;
        let target_folder = split.next().filter(|v| !v.is_empty()).map(PathBuf::from);

        Ok(BuildSettings {
            lib_path,
            watch_folder,
            manifest,
            package,
            features,
            target_folder,
            out_target,
            ..Default::default()
        })
    }
}

#[cfg(feature = "cli")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "cli")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HotReloadMessage {
    RootLibPath(String),
    UpdatedLibs(Vec<(String, [u8; 32])>),
    UpdatedAssets(Vec<(String, [u8; 32])>),
}

#[cfg(not(feature = "cli"))]
#[derive(Debug, Clone)]
pub enum HotReloadMessage {
    RootLibPath(String),
    UpdatedLibs(Vec<(String, [u8; 32])>),
    UpdatedAssets(Vec<(String, [u8; 32])>),
}
