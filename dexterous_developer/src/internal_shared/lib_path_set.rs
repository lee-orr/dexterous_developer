use std::{path::PathBuf, str::FromStr};

use crate::HotReloadOptions;

#[derive(Debug, Clone)]
pub struct LibPathSet {
    path: PathBuf,
}

impl LibPathSet {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn library_path(&self) -> PathBuf {
        self.path.clone()
    }
}
