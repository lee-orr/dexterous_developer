use anyhow::Error;
use dexterous_developer_types::HotReloadMessage;
use std::{fmt::Display, path::PathBuf, sync::Arc};

#[derive(Clone, Default)]
pub struct BuildSettings {
    pub watch_folders: Vec<PathBuf>,
    pub manifest: Option<PathBuf>,
    pub lib_path: PathBuf,
    pub package: PackageOrExample,
    pub features: String,
    pub target_folder: Option<PathBuf>,
    pub out_target: PathBuf,
    pub build_target: Option<dexterous_developer_types::Target>,
    pub updated_file_channel: Option<Arc<dyn Fn(HotReloadMessage) + Send + Sync>>,
}

impl std::fmt::Debug for BuildSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuildSettings")
            .field("watch_folders", &self.watch_folders)
            .field("manifest", &self.manifest)
            .field("lib_path", &self.lib_path)
            .field("package", &self.package)
            .field("features", &self.features)
            .field("target_folder", &self.target_folder)
            .field("out_target", &self.out_target)
            .field("build_target", &self.build_target)
            .field(
                "updated_file_channel",
                &self.updated_file_channel.as_ref().map(|_| "Has Channel"),
            )
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum PackageOrExample {
    Package(String),
    Example(String),
}

impl Default for PackageOrExample {
    fn default() -> Self {
        Self::Package(String::default())
    }
}