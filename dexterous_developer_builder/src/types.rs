use std::sync::Arc;

use camino::Utf8PathBuf;

use dashmap::DashMap;
use dexterous_developer_types::Target;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub trait Builder: 'static + Send + Sync {
    fn target(&self) -> Target;
    fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages>;
    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
        tokio::sync::broadcast::Receiver<BuildOutputMessages>,
    );
    fn root_lib_name(&self) -> Option<Utf8PathBuf>;
    fn get_watcher_subscriptions(&self) -> Vec<Utf8PathBuf>;
}

pub trait Watcher: 'static + Send + Sync {
    fn watch_directories(
        &self,
        directory: &[Utf8PathBuf],
        subscriber: tokio::sync::mpsc::Sender<BuilderIncomingMessages>,
    );
}

#[derive(Debug, Clone)]
pub enum BuilderIncomingMessages {
    RequestBuild,
    CodeChanged,
    AssetChanged(HashedFileRecord),
}

#[derive(Debug, Clone)]
pub enum BuilderOutgoingMessages {
    Waiting,
    BuildStarted,
}

#[derive(Clone, Debug)]
pub struct CurrentBuildState {
    pub root_library: Arc<Mutex<Option<Utf8PathBuf>>>,
    pub libraries: DashMap<Utf8PathBuf, HashedFileRecord>,
    pub assets: DashMap<Utf8PathBuf, HashedFileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedFileRecord {
    pub relative_path: Utf8PathBuf,
    pub local_path: Utf8PathBuf,
    pub hash: [u8; 32],
}

impl HashedFileRecord {
    pub fn new(
        relative_path: impl Into<Utf8PathBuf>,
        local_path: impl Into<Utf8PathBuf>,
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
    RootLibraryName(Utf8PathBuf),
    LibraryUpdated(HashedFileRecord),
    AssetUpdated(HashedFileRecord),
    KeepAlive,
}

impl CurrentBuildState {
    pub fn new(root_library: Option<Utf8PathBuf>) -> Self {
        Self {
            root_library: Arc::new(Mutex::new(root_library)),
            libraries: Default::default(),
            assets: Default::default(),
        }
    }

    pub async fn update(&self, msg: BuildOutputMessages) -> &Self {
        match msg {
            BuildOutputMessages::LibraryUpdated(record) => {
                self.libraries.insert(record.relative_path.clone(), record);
            }
            BuildOutputMessages::AssetUpdated(record) => {
                self.assets.insert(record.relative_path.clone(), record);
            }
            BuildOutputMessages::RootLibraryName(name) => {
                let mut lock = self.root_library.lock().await;
                let _ = lock.replace(name);
            }
            BuildOutputMessages::KeepAlive => {}
        }
        self
    }
}
