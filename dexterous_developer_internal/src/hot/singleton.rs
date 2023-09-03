use std::sync::{Once, OnceLock};

use anyhow::Error;

use crate::internal_shared::LibPathSet;

use super::build_settings::BuildSettings;

pub(crate) fn load_build_settings(settings: String) -> anyhow::Result<LibPathSet> {
    let settings = BuildSettings::try_from(settings.as_str())?;
    let lib_path = settings.lib_path.clone();
    BUILD_SETTINGS
        .set(settings)
        .map_err(|_| Error::msg("Build settings already set"))?;
    Ok(LibPathSet::new(lib_path))
}

pub(crate) static WATCHER: Once = Once::new();

pub(crate) static BUILD_SETTINGS: OnceLock<BuildSettings> = OnceLock::new();
