use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

use camino::{FromPathBufError, Utf8PathBuf};

use dashmap::DashMap;
use dexterous_developer_types::{BuilderTypes, Target};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

pub trait BuilderInitializer: 'static + Send + Sync {
    type Inner : Builder;
    
    fn initialize_builder(self, channel: tokio::sync::broadcast::Sender<BuilderIncomingMessages>) ->  anyhow::Result<Self::Inner>;
}

pub trait Builder: 'static + Send + Sync {
    fn target(&self) -> Target;
    fn builder_type(&self) -> BuilderTypes;
    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
        tokio::sync::broadcast::Receiver<BuildOutputMessages>,
    );
    fn root_lib_name(&self) -> Option<String>;
    fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf>;
    fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf>;
}

pub trait Watcher: 'static + Send + Sync {
    fn watch_code_directories(
        &self,
        directories: &[Utf8PathBuf],
    ) -> Result<(), WatcherError>;
    fn watch_asset_directories(
        &self,
        directories: &[Utf8PathBuf],
    ) -> Result<(), WatcherError>;
    fn get_channel(&self) -> tokio::sync::broadcast::Sender<BuilderIncomingMessages>;
}

#[derive(Error, Debug)]
pub enum WatcherError {
    #[error("Io Error {0}")]
    IoError(#[from] std::io::Error),
    #[error("Couldn't Find Path")]
    PathNotFound,
    #[error("Other Watch Error: {0}")]
    OtherError(String),
    #[error("Notify Error {0}")]
    NotifyError(#[from] notify::Error),
    #[error("Couldn't Parse Path Buf {0}")]
    Utf8PathBufError(#[from] FromPathBufError),
    #[error("Path is not a file {0}")]
    NotAFile(Utf8PathBuf),
}

#[derive(Debug, Clone)]
pub enum BuilderIncomingMessages {
    RequestBuild(Target),
    CodeChanged,
    AssetChanged(HashedFileRecord),
}

#[derive(Debug, Clone)]
pub enum BuilderOutgoingMessages {
    Waiting,
    BuildStarted,
}

#[derive(Clone, Debug, Default)]
pub struct CurrentBuildState {
    pub root_library: Arc<Mutex<Option<String>>>,
    pub libraries: DashMap<Utf8PathBuf, HashedFileRecord>,
    pub assets: DashMap<Utf8PathBuf, HashedFileRecord>,
    pub most_recent_completed_build: Arc<AtomicU32>,
    pub most_recent_started_build: Arc<AtomicU32>,
    pub builder_type: BuilderTypes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedFileRecord {
    pub relative_path: Utf8PathBuf,
    pub name: String,
    pub local_path: Utf8PathBuf,
    pub hash: [u8; 32],
    pub dependencies: Vec<String>,
}

impl HashedFileRecord {
    pub fn new(
        relative_path: impl Into<Utf8PathBuf>,
        local_path: impl Into<Utf8PathBuf>,
        name: impl ToString,
        hash: [u8; 32],
    ) -> Self {
        Self {
            relative_path: relative_path.into(),
            local_path: local_path.into(),
            name: name.to_string(),
            hash,
            dependencies: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildOutputMessages {
    StartedBuild(u32),
    EndedBuild {
        id: u32,
        libraries: Vec<HashedFileRecord>,
        root_library: String,
    },
    AssetUpdated(HashedFileRecord),
    KeepAlive,
}

impl CurrentBuildState {
    pub fn new(root_library: Option<String>, builder_type: BuilderTypes) -> Self {
        Self {
            root_library: Arc::new(Mutex::new(root_library)),
            libraries: Default::default(),
            assets: Default::default(),
            most_recent_completed_build: Arc::new(AtomicU32::new(0)),
            most_recent_started_build: Arc::new(AtomicU32::new(0)),
            builder_type,
        }
    }

    pub async fn update(&self, msg: BuildOutputMessages) -> &Self {
        match msg {
            BuildOutputMessages::AssetUpdated(record) => {
                self.assets.insert(record.relative_path.clone(), record);
            }
            BuildOutputMessages::KeepAlive => {}
            BuildOutputMessages::StartedBuild(id) => {
                self.most_recent_started_build
                    .fetch_max(id, Ordering::SeqCst);
            }
            BuildOutputMessages::EndedBuild {
                id,
                libraries,
                root_library,
            } => {
                for record in libraries.into_iter() {
                    self.libraries.insert(record.relative_path.clone(), record);
                }
                self.most_recent_completed_build
                    .fetch_max(id, Ordering::SeqCst);
                let mut lock = self.root_library.lock().await;
                let _ = lock.replace(root_library);
            }
        }
        self
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::Ordering;

    use camino::Utf8PathBuf;

    use super::{BuildOutputMessages, CurrentBuildState, HashedFileRecord};

    #[tokio::test]
    async fn current_build_state_can_update_asset_record() {
        let state = CurrentBuildState::default();

        let _ = state
            .update(BuildOutputMessages::AssetUpdated(HashedFileRecord {
                relative_path: Utf8PathBuf::from("/relative/path"),
                name: "asset".to_string(),
                local_path: Utf8PathBuf::from("/local/path"),
                hash: Default::default(),
                dependencies: vec![],
            }))
            .await;

        let record = state
            .assets
            .get(&Utf8PathBuf::from("/relative/path"))
            .expect("Library wasn't added to current build state");
        assert_eq!(record.name, "asset");
        assert_eq!(record.local_path.as_str(), "/local/path");
    }

    #[tokio::test]
    async fn starting_a_new_build_updates_current_state() {
        let state = CurrentBuildState::default();

        let _ = state.update(BuildOutputMessages::StartedBuild(1)).await;

        assert_eq!(state.most_recent_started_build.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn starting_a_previous_build_after_a_newer_one_doesnt_update_current_state() {
        let state = CurrentBuildState::default();

        let _ = state.update(BuildOutputMessages::StartedBuild(2)).await;

        assert_eq!(state.most_recent_started_build.load(Ordering::SeqCst), 2);

        let _ = state.update(BuildOutputMessages::StartedBuild(1)).await;
        assert_eq!(state.most_recent_started_build.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn ending_a_build_updates_current_state() {
        let state = CurrentBuildState::default();

        let _ = state
            .update(BuildOutputMessages::EndedBuild {
                id: 1,
                libraries: vec![HashedFileRecord {
                    relative_path: Utf8PathBuf::from("/relative/path"),
                    name: "library".to_string(),
                    local_path: Utf8PathBuf::from("/local/path"),
                    hash: Default::default(),
                    dependencies: vec![],
                }],
                root_library: "Root".to_string(),
            })
            .await;

        let record = state
            .libraries
            .get(&Utf8PathBuf::from("/relative/path"))
            .expect("Library wasn't added to current build state");
        assert_eq!(record.name, "library");
        assert_eq!(record.local_path.as_str(), "/local/path");

        let read = state.root_library.lock().await;
        let read = read.as_ref().map(|v| v.clone()).unwrap();
        assert_eq!(read, "Root");

        assert_eq!(state.most_recent_completed_build.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn ending_a_previous_build_after_a_newer_one_doesnt_update_current_state() {
        let state = CurrentBuildState::default();

        let _ = state
            .update(BuildOutputMessages::EndedBuild {
                id: 2,
                libraries: vec![],
                root_library: String::default(),
            })
            .await;

        assert_eq!(state.most_recent_completed_build.load(Ordering::SeqCst), 2);

        let _ = state
            .update(BuildOutputMessages::EndedBuild {
                id: 1,
                libraries: vec![],
                root_library: String::default(),
            })
            .await;
        assert_eq!(state.most_recent_completed_build.load(Ordering::SeqCst), 2);
    }
}
