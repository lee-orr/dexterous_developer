use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct HotReloadOptions {
    pub package: Option<String>,
    pub lib_name: Option<String>,
    pub watch_folder: Option<PathBuf>,
    pub target_folder: Option<PathBuf>,
    pub features: Vec<String>,
    pub set_env: bool,
}
