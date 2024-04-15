use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;
use dexterous_developer_builder::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
    CurrentBuildState, Watcher,
};
use dexterous_developer_types::Target;
use std::{collections::HashSet, sync::Arc};
use thiserror::Error;
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tracing::info;

#[derive(Default, Clone)]

pub struct Manager {
    targets: Arc<
        DashMap<
            Target,
            (
                mpsc::UnboundedSender<BuilderIncomingMessages>,
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

#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("Can't build  target {0}")]
    MissingTarget(Target),
    #[error("Can't subscribe to target {0}")]
    SubscriptionFailed(Target),
    #[error("Failed to receive message {0}")]
    ReceiveError(#[from] tokio::sync::broadcast::error::RecvError),
}

impl Manager {
    pub fn new(watcher: Arc<dyn Watcher>) -> Self {
        Manager {
            targets: Default::default(),
            watcher: Some(watcher),
            target_count: 0,
        }
    }

    pub async fn add_builders(mut self, builders: &[Arc<dyn Builder>]) -> Self {
        for builder in builders.iter() {
            self.target_count += 1;
            let id = self.target_count;
            let target = builder.target();
            self.targets.entry(target).or_insert_with(|| {
                let current_state = Arc::new(CurrentBuildState::new(builder.root_lib_name()));
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
                                        BuilderOutgoingMessages::Waiting => info!("Builder for {target:?} is waiting"),
                                        BuilderOutgoingMessages::BuildStarted => info!("Started building for {target:?}"),
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

                let incoming = builder.incoming_channel();

                if let Some(watcher) = &self.watcher {
                    let _ = watcher.watch_code_directories(&builder.get_code_subscriptions(), (id, incoming.clone()));
                    let _ = watcher.watch_asset_directories(&builder.get_asset_subscriptions(), (id, incoming.clone()));
                }

                (incoming, outgoing, output, current_state, handle)
            });
        }
        self
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

        let (sender, _receiver, output_rx, current_state, _) = target_ref.value();

        let response = (current_state.as_ref().clone(), output_rx.resubscribe());

        let _ = sender.send(BuilderIncomingMessages::RequestBuild);
        Ok(response)
    }

    pub fn get_filepath(
        &self,
        target: &Target,
        path: &Utf8Path,
    ) -> Result<Option<Utf8PathBuf>, ManagerError> {
        let target_ref = self
            .targets
            .get(target)
            .ok_or(ManagerError::MissingTarget(*target))?;

        let current_state = &target_ref.3;

        let file = current_state
            .libraries
            .get(path)
            .or_else(|| current_state.assets.get(path));

        Ok(file.map(|v| v.local_path.clone()))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use dexterous_developer_builder::types::{
        Builder, BuilderIncomingMessages, BuilderOutgoingMessages, HashedFileRecord, WatcherError,
    };

    struct TestBuilder;

    impl Builder for TestBuilder {
        fn target(&self) -> Target {
            Target::Android
        }

        fn incoming_channel(&self) -> tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages> {
            mpsc::unbounded_channel().0
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Option<Utf8PathBuf> {
            Some(Utf8PathBuf::from("root_lib"))
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }
    }

    struct TestBuilder2;

    impl Builder for TestBuilder2 {
        fn target(&self) -> Target {
            Target::IOS
        }

        fn incoming_channel(&self) -> tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages> {
            mpsc::unbounded_channel().0
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Option<Utf8PathBuf> {
            Some(Utf8PathBuf::from("root_lib"))
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }
    }

    #[tokio::test]
    async fn when_provided_with_builders_can_return_their_targets() {
        let manager = Manager::default()
            .add_builders(&[Arc::new(TestBuilder), Arc::new(TestBuilder2)])
            .await;

        let targets = manager.targets();

        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&Target::Android));
        assert!(targets.contains(&Target::IOS));
    }

    #[tokio::test]
    async fn requesting_a_missing_target_returns_an_error() {
        let manager = Manager::default()
            .add_builders(&[Arc::new(TestBuilder)])
            .await;

        let err = manager
            .watch_target(&Target::IOS)
            .await
            .expect_err("Didn't fail to watch target");

        assert!(matches!(err, ManagerError::MissingTarget(Target::IOS)));
    }

    struct TestChanneledBuilder {
        target: Target,
        incoming: tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages>,
        outgoing: tokio::sync::broadcast::Sender<BuilderOutgoingMessages>,
        output: tokio::sync::broadcast::Sender<BuildOutputMessages>,
        #[allow(dead_code)]
        handle: tokio::task::JoinHandle<()>,
    }

    impl TestChanneledBuilder {
        fn new(target: Target) -> Self {
            let (incoming, mut incoming_rx) = tokio::sync::mpsc::unbounded_channel();
            let (outgoing_tx, _) = tokio::sync::broadcast::channel(10);
            let (output_tx, _) = tokio::sync::broadcast::channel(10);
            let _my_target = target;

            let handle = {
                let outgoing_tx = outgoing_tx.clone();
                let output_tx = output_tx.clone();
                tokio::spawn(async move {
                    while let Some(recv) = incoming_rx.recv().await {
                        if let BuilderIncomingMessages::RequestBuild = recv {
                            if outgoing_tx
                                .send(BuilderOutgoingMessages::BuildStarted)
                                .is_err()
                            {
                                break;
                            }
                            if output_tx
                                .send(BuildOutputMessages::LibraryUpdated(HashedFileRecord::new(
                                    "root_lib_path",
                                    Utf8PathBuf::new(),
                                    [
                                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                    ],
                                )))
                                .is_err()
                            {
                                break;
                            }
                        }
                        if let BuilderIncomingMessages::CodeChanged = recv {
                            output_tx
                                .send(BuildOutputMessages::LibraryUpdated(HashedFileRecord::new(
                                    "root_lib_path",
                                    Utf8PathBuf::new(),
                                    [
                                        1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                    ],
                                )))
                                .expect("Failed to send watch");
                        }
                    }
                })
            };

            Self {
                target,
                incoming,
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

        fn incoming_channel(&self) -> tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages> {
            self.incoming.clone()
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (self.outgoing.subscribe(), self.output.subscribe())
        }

        fn root_lib_name(&self) -> Option<Utf8PathBuf> {
            Some(Utf8PathBuf::from("root_lib"))
        }

        fn get_code_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![Utf8PathBuf::from("watched_path")]
        }

        fn get_asset_subscriptions(&self) -> Vec<Utf8PathBuf> {
            vec![]
        }
    }

    #[tokio::test]
    async fn watching_a_target_returns_a_reciever_for_the_first_matching_builder() {
        let builder_1 = Arc::new(TestChanneledBuilder::new(Target::Android));
        let builder_2 = Arc::new(TestChanneledBuilder::new(Target::Android));

        let channel = builder_1.incoming_channel();

        let manager = Manager::default()
            .add_builders(&[builder_1, builder_2])
            .await;

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

            let _ = channel.send(BuilderIncomingMessages::RequestBuild);

            let message = rx.recv().await.unwrap();
            match message {
                BuildOutputMessages::LibraryUpdated(HashedFileRecord {
                    relative_path,
                    hash,
                    ..
                }) => {
                    assert_eq!(relative_path.to_string(), "root_lib_path");
                    hash
                }
                _ => panic!("Message is wrong type"),
            }
        };
        {
            let (current_state, _rx) = manager
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
        }
    }

    struct TestWatcher {
        subscribers:
            DashMap<Utf8PathBuf, tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages>>,
    }

    impl TestWatcher {
        fn new() -> Self {
            TestWatcher {
                subscribers: Default::default(),
            }
        }

        async fn update(&self, directory: Utf8PathBuf) {
            if let Some(sub) = self.subscribers.get(&directory) {
                let _ = sub.send(BuilderIncomingMessages::CodeChanged);
            }
        }
    }

    impl Watcher for TestWatcher {
        fn watch_code_directories(
            &self,
            directories: &[Utf8PathBuf],
            (_, subscriber): (
                usize,
                tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages>,
            ),
        ) -> Result<(), WatcherError> {
            for directory in directories.iter() {
                self.subscribers
                    .insert(directory.clone(), subscriber.clone());
            }
            Ok(())
        }

        fn watch_asset_directories(
            &self,
            _directory: &[Utf8PathBuf],
            (_, _subscriber): (
                usize,
                tokio::sync::mpsc::UnboundedSender<BuilderIncomingMessages>,
            ),
        ) -> Result<(), WatcherError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn given_a_watcher_subscribes_builders_correctly() {
        let builder_1 = Arc::new(TestChanneledBuilder::new(Target::Android));

        let channel = builder_1.incoming_channel();
        let watcher = Arc::new(TestWatcher::new());

        let manager = Manager::new(watcher.clone())
            .add_builders(&[builder_1])
            .await;

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

            let _ = channel.send(BuilderIncomingMessages::RequestBuild);

            let message = rx.recv().await.unwrap();
            match message {
                BuildOutputMessages::LibraryUpdated(HashedFileRecord {
                    relative_path,
                    hash,
                    ..
                }) => {
                    assert_eq!(relative_path.to_string(), "root_lib_path");
                    hash
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

            watcher.update(Utf8PathBuf::from("watched_path")).await;

            let message = rx.recv().await.unwrap();
            let new_hash = match message {
                BuildOutputMessages::LibraryUpdated(HashedFileRecord {
                    relative_path,
                    hash,
                    ..
                }) => {
                    assert_eq!(relative_path.to_string(), "root_lib_path");
                    hash
                }
                _ => panic!("Message is wrong type"),
            };

            assert!(hash != new_hash, "Original: {hash:?}, new: {new_hash:?}");
        }
    }
}
