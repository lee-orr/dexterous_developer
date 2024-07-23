use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;
use dexterous_developer_builder::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderInitializer,
    BuilderOutgoingMessages, CurrentBuildState, Watcher,
};
use dexterous_developer_types::Target;
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use tokio::{
    sync::broadcast::{self},
    task::JoinHandle,
};
use tracing::{error, info, trace};

#[derive(Clone)]

pub struct Manager {
    watcher_channel: broadcast::Sender<BuilderIncomingMessages>,
    targets: Arc<
        DashMap<
            Target,
            (
                broadcast::Receiver<BuilderOutgoingMessages>,
                broadcast::Receiver<BuildOutputMessages>,
                Arc<CurrentBuildState>,
                JoinHandle<()>,
            ),
        >,
    >,
    target_count: usize,
    watcher: Option<Arc<dyn Watcher>>,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            watcher_channel: broadcast::channel(100).0,
            targets: Default::default(),
            target_count: Default::default(),
            watcher: Default::default(),
        }
    }
}
#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("Can't build  target {0}")]
    MissingTarget(Target),
    #[error("Can't subscribe to target {0}")]
    SubscriptionFailed(Target),
    #[error("Failed to receive message {0}")]
    ReceiveError(#[from] tokio::sync::broadcast::error::RecvError),
    #[error("Requested File Isn't Available")]
    NoSuchFile(Utf8PathBuf),
}

impl Manager {
    pub fn new(watcher: Arc<dyn Watcher>) -> Self {
        let watcher_channel = watcher.get_channel();

        Manager {
            watcher_channel,
            targets: Default::default(),
            watcher: Some(watcher),
            target_count: 0,
        }
    }

    pub fn get_watcher_channel(&self) -> broadcast::Sender<BuilderIncomingMessages> {
        self.watcher_channel.clone()
    }

    pub fn add_builder<Initializer: BuilderInitializer>(
        mut self,
        initializer: Initializer,
    ) -> anyhow::Result<Self> {
        let builder = initializer.initialize_builder(self.watcher_channel.clone())?;
        let target = builder.target();
        self.targets.entry(target).or_insert_with(|| {
            self.target_count += 1;
            let current_state = Arc::new(CurrentBuildState::new(builder.root_lib_name(), builder.builder_type()));
            let (outgoing, output) = builder.outgoing_channel();

            let handle = {
                let mut outgoing = outgoing.resubscribe();
                let mut output = output.resubscribe();
                let current_state = current_state.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            Ok(msg) = outgoing.recv() => {
                                match msg {
                                    BuilderOutgoingMessages::Waiting => trace!("Builder for {target:?} is waiting"),
                                    BuilderOutgoingMessages::BuildStarted => trace!("Started building for {target:?}"),
                                }
                            }
                            Ok(msg) = output.recv() => {
                                current_state.update(msg).await;
                            }
                            else => { break }
                        }
                    }
                })
            };

            if let Some(watcher) = &self.watcher {
                let _ = watcher.watch_code_directories(&builder.get_code_subscriptions());
                let _ = watcher.watch_asset_directories(&builder.get_asset_subscriptions());
            }

            (outgoing, output, current_state, handle)
        });

        let targets = self.targets.iter().map(|r| *r.key()).collect::<Vec<_>>();
        info!("Able to build {targets:?}");
        Ok(self)
    }

    pub fn targets(&self) -> HashSet<Target> {
        self.targets.iter().map(|key| *key.key()).collect()
    }

    pub async fn watch_target(
        &self,
        target: &Target,
    ) -> Result<(CurrentBuildState, broadcast::Receiver<BuildOutputMessages>), ManagerError> {
        let target_ref = self
            .targets
            .get(target)
            .ok_or(ManagerError::MissingTarget(*target))?;

        let (_, output_rx, current_state, _) = target_ref.value();

        let response = (current_state.as_ref().clone(), output_rx.resubscribe());

        let _ = self
            .watcher_channel
            .send(BuilderIncomingMessages::RequestBuild(*target));
        Ok(response)
    }

    pub fn get_filepath(
        &self,
        target: &Target,
        path: &Utf8Path,
    ) -> Result<Utf8PathBuf, ManagerError> {
        let target_ref = self
            .targets
            .get(target)
            .ok_or(ManagerError::MissingTarget(*target))?;

        let current_state = &target_ref.2;

        let file = current_state
            .libraries
            .get(path)
            .or_else(|| current_state.assets.get(path))
            .ok_or_else(|| {
                error!("Known Libraries: {:?}", current_state.libraries);
                ManagerError::NoSuchFile(path.to_owned())
            })?;

        Ok(file.local_path.clone())
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use dexterous_developer_builder::types::{
        Builder, BuilderIncomingMessages, BuilderOutgoingMessages, HashedFileRecord, WatcherError,
    };

    struct TestBuilderInitializer;

    impl BuilderInitializer for TestBuilderInitializer {
        type Inner = TestBuilder;

        fn initialize_builder(
            self,
            _: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
        ) -> anyhow::Result<Self::Inner> {
            Ok(TestBuilder)
        }
    }

    struct TestBuilder;

    impl Builder for TestBuilder {
        fn target(&self) -> Target {
            Target::Android
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Option<String> {
            Some("root_lib".to_string())
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn builder_type(&self) -> dexterous_developer_types::BuilderTypes {
            dexterous_developer_types::BuilderTypes::Simple
        }
    }

    struct TestBuilderInitializer2;

    impl BuilderInitializer for TestBuilderInitializer2 {
        type Inner = TestBuilder2;

        fn initialize_builder(
            self,
            _: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
        ) -> anyhow::Result<Self::Inner> {
            Ok(TestBuilder2)
        }
    }

    struct TestBuilder2;

    impl Builder for TestBuilder2 {
        fn target(&self) -> Target {
            Target::IOS
        }
        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Option<String> {
            Some("root_lib".to_string())
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn builder_type(&self) -> dexterous_developer_types::BuilderTypes {
            dexterous_developer_types::BuilderTypes::Simple
        }
    }

    #[tokio::test]
    async fn when_provided_with_builders_can_return_their_targets() {
        let manager = Manager::default()
            .add_builder(TestBuilderInitializer)
            .expect("Couldn't initialize builder")
            .add_builder(TestBuilderInitializer2)
            .expect("Couldn't Initialize Second Builder");

        let targets = manager.targets();

        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&Target::Android));
        assert!(targets.contains(&Target::IOS));
    }

    #[tokio::test]
    async fn requesting_a_missing_target_returns_an_error() {
        let manager = Manager::default()
            .add_builder(TestBuilderInitializer)
            .expect("Couldn't initialize builder");

        let err = manager
            .watch_target(&Target::IOS)
            .await
            .expect_err("Didn't fail to watch target");

        assert!(matches!(err, ManagerError::MissingTarget(Target::IOS)));
    }

    struct TestChanneledBuilderInitializer {
        target: Target,
    }

    impl TestChanneledBuilderInitializer {
        fn new(target: Target) -> Self {
            Self { target }
        }
    }

    impl BuilderInitializer for TestChanneledBuilderInitializer {
        type Inner = TestChanneledBuilder;

        fn initialize_builder(
            self,
            channel: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
        ) -> Result<TestChanneledBuilder, anyhow::Error> {
            Ok(Self::Inner::new(self.target, channel))
        }
    }

    struct TestChanneledBuilder {
        target: Target,
        outgoing: tokio::sync::broadcast::Sender<BuilderOutgoingMessages>,
        output: tokio::sync::broadcast::Sender<BuildOutputMessages>,
        #[allow(dead_code)]
        handle: tokio::task::JoinHandle<()>,
    }

    impl TestChanneledBuilder {
        fn new(
            target: Target,
            incoming: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
        ) -> Self {
            let mut incoming_rx = incoming.subscribe();
            let (outgoing_tx, _) = tokio::sync::broadcast::channel(10);
            let (output_tx, _) = tokio::sync::broadcast::channel(10);
            let _my_target = target;

            let handle = {
                let outgoing_tx = outgoing_tx.clone();
                let output_tx = output_tx.clone();
                tokio::spawn(async move {
                    while let Ok(recv) = incoming_rx.recv().await {
                        if let BuilderIncomingMessages::RequestBuild(req) = recv {
                            if req != target {
                                continue;
                            }
                            if outgoing_tx
                                .send(BuilderOutgoingMessages::BuildStarted)
                                .is_err()
                            {
                                break;
                            }
                            if output_tx
                                .send(BuildOutputMessages::EndedBuild {
                                    libraries: vec![HashedFileRecord::new(
                                        "root_lib_path",
                                        Utf8PathBuf::new(),
                                        "root_lib_path",
                                        [
                                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                        ],
                                    )],
                                    id: 1,
                                    root_library: "root_lib".to_string(),
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        if let BuilderIncomingMessages::CodeChanged = recv {
                            output_tx
                                .send(BuildOutputMessages::EndedBuild {
                                    libraries: vec![HashedFileRecord::new(
                                        "root_lib_path",
                                        Utf8PathBuf::new(),
                                        "root_lib_path",
                                        [
                                            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                        ],
                                    )],
                                    id: 1,
                                    root_library: "root_lib".to_string(),
                                })
                                .expect("Failed to send watch");
                        }
                    }
                })
            };

            Self {
                target,
                outgoing: outgoing_tx,
                output: output_tx,
                handle,
            }
        }
    }

    impl Builder for TestChanneledBuilder {
        fn target(&self) -> Target {
            self.target
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (self.outgoing.subscribe(), self.output.subscribe())
        }

        fn root_lib_name(&self) -> Option<String> {
            Some("root_lib".to_string())
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![Utf8PathBuf::from("watched_path")]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn builder_type(&self) -> dexterous_developer_types::BuilderTypes {
            dexterous_developer_types::BuilderTypes::Simple
        }
    }

    struct TestWatcher {
        channel: tokio::sync::broadcast::Sender<BuilderIncomingMessages>,
    }

    impl TestWatcher {
        fn new() -> Self {
            TestWatcher {
                channel: broadcast::channel(100).0,
            }
        }

        async fn update(&self) {
            let _ = self.channel.send(BuilderIncomingMessages::CodeChanged);
        }
    }

    impl Watcher for TestWatcher {
        fn watch_code_directories(&self, _directories: &[Utf8PathBuf]) -> Result<(), WatcherError> {
            Ok(())
        }

        fn watch_asset_directories(&self, _directory: &[Utf8PathBuf]) -> Result<(), WatcherError> {
            Ok(())
        }

        fn get_channel(&self) -> tokio::sync::broadcast::Sender<BuilderIncomingMessages> {
            self.channel.clone()
        }
    }

    #[tokio::test]
    async fn given_a_watcher_subscribes_builders_correctly() {
        let builder_1 = TestChanneledBuilderInitializer::new(Target::Android);

        let watcher = Arc::new(TestWatcher::new());

        let channel = watcher.channel.clone();

        let manager = Manager::new(watcher.clone())
            .add_builder(builder_1)
            .expect("Failed to add builder");

        let hash = {
            let (current_state, mut rx) = manager
                .watch_target(&Target::Android)
                .await
                .expect("Failed to watch target");

            assert_eq!(
                {
                    let lock = current_state.root_library.lock().await;
                    lock.as_ref().unwrap().clone()
                },
                Utf8PathBuf::from("root_lib")
            );
            assert!(current_state
                .libraries
                .get(&Utf8PathBuf::from("root_lib_path"))
                .is_none());

            let _ = channel.send(BuilderIncomingMessages::RequestBuild(Target::Android));

            let message = rx.recv().await.unwrap();
            match message {
                BuildOutputMessages::EndedBuild { libraries, .. } => {
                    let Some(HashedFileRecord {
                        relative_path,
                        hash,
                        ..
                    }) = libraries.first()
                    else {
                        panic!("No Updated Libraries");
                    };
                    assert_eq!(relative_path.to_string(), "root_lib_path");
                    *hash
                }
                _ => panic!("Message is wrong type"),
            }
        };
        {
            let (current_state, mut rx) = manager
                .watch_target(&Target::Android)
                .await
                .expect("Failed to watch target");

            assert_eq!(
                {
                    let lock = current_state.root_library.lock().await;
                    lock.as_ref().unwrap().clone()
                },
                Utf8PathBuf::from("root_lib")
            );

            assert_eq!(
                current_state
                    .libraries
                    .get(&Utf8PathBuf::from("root_lib_path"))
                    .unwrap()
                    .hash,
                hash
            );

            let _ = rx.recv().await;

            watcher.update().await;

            let message = rx.recv().await.unwrap();
            let new_hash = match message {
                BuildOutputMessages::EndedBuild { libraries, .. } => {
                    let Some(HashedFileRecord {
                        relative_path,
                        hash,
                        ..
                    }) = libraries.first()
                    else {
                        panic!("No Updated Libraries");
                    };
                    assert_eq!(relative_path.to_string(), "root_lib_path");
                    *hash
                }
                _ => panic!("Message is wrong type"),
            };

            assert!(hash != new_hash, "Original: {hash:?}, new: {new_hash:?}");
        }
    }
}
