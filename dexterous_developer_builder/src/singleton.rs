use std::sync::{Once, OnceLock};

use anyhow::Error;
use dexterous_developer_types::LibPathSet;

use super::build_settings::BuildSettings;

pub fn load_build_settings(settings: String) -> anyhow::Result<LibPathSet> {
    let settings = BuildSettings::try_from(settings.as_str())?;
    let lib_path = settings.lib_path.clone();
    BUILD_SETTINGS
        .set(settings)
        .map_err(|_| Error::msg("Build settings already set"))?;
    Ok(LibPathSet::new(lib_path))
}

pub static WATCHER: Once = Once::new();

pub static BUILD_SETTINGS: OnceLock<BuildSettings> = OnceLock::new();
