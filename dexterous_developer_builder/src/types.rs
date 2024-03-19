use std::path::PathBuf;

use dashmap::DashMap;
use dexterous_developer_types::Target;
use serde::{Deserialize, Serialize};

pub trait Builder: 'static + Send + Sync {
    fn target(&self) -> Target;
    fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages>;
    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
        tokio::sync::broadcast::Receiver<BuildOutputMessages>,
    );
    fn root_lib_name(&self) -> PathBuf;
}

#[derive(Debug, Clone)]
pub enum BuilderIncomingMessages {
    RequestBuild,
}

#[derive(Debug, Clone)]
pub enum BuilderOutgoingMessages {
    Waiting,
    BuildStarted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentBuildState {
    pub root_library: PathBuf,
    pub libraries: DashMap<PathBuf, HashedFileRecord>,
    pub assets: DashMap<PathBuf, HashedFileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedFileRecord {
    pub relative_path: PathBuf,
    pub local_path: PathBuf,
    pub hash: [u8; 32],
}

impl HashedFileRecord {
    pub fn new(
        relative_path: impl Into<PathBuf>,
        local_path: impl Into<PathBuf>,
        hash: [u8; 32],
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            local_path: local_path.into(),
            hash,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildOutputMessages {
    RootLibraryName(PathBuf),
    LibraryUpdated(HashedFileRecord),
    AssetUpdated(HashedFileRecord),
    KeepAlive,
}

impl CurrentBuildState {
    pub fn new(root_library: PathBuf) -> Self {
        Self {
            root_library,
            libraries: Default::default(),
            assets: Default::default(),
        }
    }

    pub fn update(&self, msg: BuildOutputMessages) -> &Self {
        match msg {
            BuildOutputMessages::LibraryUpdated(record) => {
                self.libraries.insert(record.relative_path.clone(), record);
            }
            BuildOutputMessages::AssetUpdated(record) => {
                self.assets.insert(record.relative_path.clone(), record);
            }
            BuildOutputMessages::RootLibraryName(_) | BuildOutputMessages::KeepAlive => {}
        }
        self
    }
}
