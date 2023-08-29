use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct HotReloadOptions {
    pub manifest_path: Option<PathBuf>,
    pub package: Option<String>,
    pub lib_name: Option<String>,
    pub watch_folder: Option<PathBuf>,
    pub target_folder: Option<PathBuf>,
    pub features: Vec<String>,
    pub set_env: bool,
    pub prefer_mold: bool,
}
