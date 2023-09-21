use std::{fmt::Display, ops::Deref, path::PathBuf, str::FromStr};

use anyhow::bail;

#[derive(Debug, Default)]
pub struct HotReloadOptions {
    pub manifest_path: Option<PathBuf>,
    pub package: Option<String>,
    pub lib_name: Option<String>,
    pub watch_folders: Vec<PathBuf>,
    pub target_folder: Option<PathBuf>,
    pub features: Vec<String>,
    pub set_env: bool,
    pub build_target: Option<Target>,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Target {
    Linux,
    LinuxArm,
    Windows,
    Mac,
    MacArm,
}

#[cfg(any(feature = "cli", feature = "bevy"))]
pub use serialize::*;

#[cfg(any(feature = "cli", feature = "bevy"))]
mod serialize {
    use super::*;
    use serde::{
        de::{self},
        Deserialize, Deserializer, Serialize,
    };

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
}

impl Target {
    pub const fn to_static(self) -> &'static str {
        match self {
            Target::Linux => "x86_64-unknown-linux-gnu",
            Target::LinuxArm => "aarch64-unknown-linux-gnu",
            Target::Windows => {
                if cfg!(windows) {
                    "x86_64-pc-windows-msvc"
                } else {
                    "x86_64-pc-windows-gnu"
                }
            }
            Target::Mac => "x86_64-apple-darwin",
            Target::MacArm => "aarch64-apple-darwin",
        }
    }

    pub fn as_str(&self) -> &'static str {
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

impl FromStr for Target {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();
        if s.contains("windows") {
            Ok(Self::Windows)
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
        } else {
            bail!("Invalid Target");
        }
    }
}
