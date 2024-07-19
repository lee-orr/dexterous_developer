pub mod cargo_path_utils;

#[cfg(feature = "config")]
pub mod config;

use std::{collections::HashMap, fmt::Display, ops::Deref, str::FromStr};

use camino::Utf8PathBuf;
use serde::{de, Deserialize, Deserializer, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LibraryPath {
    path: Utf8PathBuf,
}

impl LibraryPath {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        debug!("Creating path");
        Self { path: path.into() }
    }

    pub fn library_path(&self) -> Utf8PathBuf {
        self.path.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Hash, PartialEq, Eq)]
pub enum PackageOrExample {
    #[default]
    DefaulPackage,
    Package(String),
    Example(String),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BuilderTypes {
    #[default]
    Simple,
    Incremental,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TargetBuildSettings {
    pub working_dir: Option<camino::Utf8PathBuf>,
    pub manifest_path: Option<Utf8PathBuf>,
    pub package_or_example: PackageOrExample,
    pub features: Vec<String>,
    pub asset_folders: Vec<camino::Utf8PathBuf>,
    pub code_watch_folders: Vec<camino::Utf8PathBuf>,
    pub environment: HashMap<String, String>,
    pub builder: BuilderTypes,
    pub additional_library_directories: Vec<Utf8PathBuf>,
    pub apple_sdk_directory: Vec<Utf8PathBuf>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Target {
    Linux,
    LinuxArm,
    Windows,
    Mac,
    MacArm,
    Android,
    IOS,
}

impl Target {
    pub fn current() -> Option<Self> {
        if cfg!(target_os = "linux") {
            if cfg!(target_arch = "aarch64") {
                Some(Self::LinuxArm)
            } else {
                Some(Self::Linux)
            }
        } else if cfg!(target_os = "windows") {
            Some(Self::Windows)
        } else if cfg!(target_os = "macos") {
            if cfg!(target_arch = "aarch64") {
                Some(Self::MacArm)
            } else {
                Some(Self::Mac)
            }
        } else {
            None
        }
    }
}

impl Serialize for Target {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self.to_static())
    }
}

impl<'de> Deserialize<'de> for Target {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        FromStr::from_str(&s).map_err(de::Error::custom)
    }
}

impl Target {
    pub const fn to_static(self) -> &'static str {
        match self {
            Target::Linux => "x86_64-unknown-linux-gnu",
            Target::LinuxArm => "aarch64-unknown-linux-gnu",
            Target::Windows => {
                "x86_64-pc-windows-gnu"
            }
            Target::Mac => "x86_64-apple-darwin",
            Target::MacArm => "aarch64-apple-darwin",
            Target::Android => "aarch64-linux-android",
            Target::IOS => "aarch64-apple-ios",
        }
    }

    pub const fn zig_rustc_target(self) -> &'static str {
        match self {
            Target::Linux => "x86_64-linux-gnu",
            Target::LinuxArm => "aarch64-linux-gnu",
            Target::Windows => "x86_64-windows-gnu",
            Target::Mac => "x86_64-macos",
            Target::MacArm => "aarch64-macos",
            Target::Android => "aarch64-android",
            Target::IOS => "aarch64-ios",
        }
    }

    pub const fn zig_linker_target(self) -> &'static str {
        match self {
            Target::Linux => "x86_64-linux-gnu",
            Target::LinuxArm => "aarch64-linux-gnu",
            Target::Windows => "x86_64-windows-gnu",
            Target::Mac => "x86_64-macos",
            Target::MacArm => "aarch64-macos",
            Target::Android => "aarch64-android",
            Target::IOS => "aarch64-ios",
        }
    }

    pub const fn dynamic_lib_extension(&self) -> &'static str {
        match self {
            Target::Windows => "dll",
            Target::Mac => "dylib",
            Target::MacArm => "dylib",
            Target::IOS => "dylib",
            _ => "so",
        }
    }

    pub const fn dynamic_lib_prefix(&self) -> &'static str {
        match self {
            Target::Windows => "",
            _ => "lib",
        }
    }

    pub fn dynamic_lib_name(&self, name: &str) -> String {
        let prefix = self.dynamic_lib_prefix();
        let extension = self.dynamic_lib_extension();
        format!("{prefix}{name}.{extension}")
    }

    pub const fn as_str(&self) -> &'static str {
        self.to_static()
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

impl Deref for Target {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum TargetParseError {
    #[error("Couldn't Parse Target")]
    InvalidTarget,
}

impl FromStr for Target {
    type Err = TargetParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();
        if s.contains("windows") {
            Ok(Self::Windows)
        } else if s.contains("android") {
            Ok(Self::Android)
        } else if s.contains("linux") {
            if s.contains("arm") || s.contains("aarch") {
                Ok(Self::LinuxArm)
            } else {
                Ok(Self::Linux)
            }
        } else if s.contains("darwin") || s.contains("mac") {
            if s.contains("arm") || s.contains("aarch") {
                Ok(Self::MacArm)
            } else {
                Ok(Self::Mac)
            }
        } else if s.contains("ios") {
            Ok(Self::IOS)
        } else {
            Err(TargetParseError::InvalidTarget)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HotReloadMessage {
    InitialState {
        id: uuid::Uuid,
        root_lib: Option<String>,
        libraries: Vec<(Utf8PathBuf, [u8; 32])>,
        assets: Vec<(Utf8PathBuf, [u8; 32])>,
        most_recent_started_build: u32,
        most_recent_completed_build: u32,
        builder_type: BuilderTypes,
    },
    UpdatedAssets(Utf8PathBuf, [u8; 32]),
    KeepAlive,
    BuildStarted(u32),
    BuildCompleted {
        id: u32,
        libraries: Vec<(String, [u8; 32], Vec<String>)>,
        root_library: String,
    },
}
