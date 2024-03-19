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
    fn root_lib_name(&self) -> String;
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
    pub root_library: String,
    pub libraries: DashMap<String, (PathBuf, String, [u8; 32])>,
    pub assets: DashMap<String, (PathBuf, String, [u8; 32])>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildOutputMessages {
    RootLibraryName(String),
    LibraryUpdated(PathBuf, String, [u8; 32]),
    AssetUpdated(PathBuf, String, [u8; 32]),
    KeepAlive,
}

impl CurrentBuildState {
    pub fn new(root_library: String) -> Self {
        Self {
            root_library,
            libraries: Default::default(),
            assets: Default::default(),
        }
    }

    pub fn update(&self, msg: BuildOutputMessages) -> &Self {
        match msg {
            BuildOutputMessages::LibraryUpdated(path, name, hash) => {
                self.libraries.insert(name.clone(), (path, name, hash));
            }
            BuildOutputMessages::AssetUpdated(path, name, hash) => {
                self.assets.insert(name.clone(), (path, name, hash));
            }
            BuildOutputMessages::RootLibraryName(_) | BuildOutputMessages::KeepAlive => {}
        }
        self
    }
}
