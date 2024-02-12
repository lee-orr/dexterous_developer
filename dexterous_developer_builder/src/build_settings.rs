use anyhow::Error;
use dexterous_developer_types::HotReloadMessage;
use std::{path::PathBuf, sync::Arc};

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

impl ToString for BuildSettings {
    fn to_string(&self) -> String {
        let BuildSettings {
            watch_folders,
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

        let watch_folder = std::env::join_paths(watch_folders)
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();
        let manifest = manifest
            .as_ref()
            .map(|v| v.to_string_lossy())
            .unwrap_or_default();
        let lib_path: std::borrow::Cow<'_, str> = lib_path.to_string_lossy();
        let package = match package {
            PackageOrExample::Package(v) => v.clone(),
            PackageOrExample::Example(v) => format!("example:{v}"),
        };

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
        let watch_folders = split
            .next()
            .map(std::env::split_paths)
            .map(|v| v.map(|v| v.to_path_buf()).collect::<Vec<_>>())
            .ok_or(Error::msg("no watch folder"))?;
        let manifest = split.next().filter(|v| !v.is_empty()).map(PathBuf::from);
        let package = split
            .next()
            .map(|v| {
                if v.starts_with("example:") {
                    PackageOrExample::Example(v.replace("example:", "").to_string())
                } else {
                    PackageOrExample::Package(v.to_string())
                }
            })
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
            watch_folders,
            manifest,
            package,
            features,
            target_folder,
            out_target,
            ..Default::default()
        })
    }
}
