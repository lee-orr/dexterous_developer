use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;
use dexterous_developer_builder::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
    CurrentBuildState,
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
                mpsc::Sender<BuilderIncomingMessages>,
                broadcast::Receiver<BuilderOutgoingMessages>,
                broadcast::Receiver<BuildOutputMessages>,
                Arc<CurrentBuildState>,
                JoinHandle<()>,
            ),
        >,
    >,
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
    pub async fn add_builders(self, builders: &[Arc<dyn Builder>]) -> Self {
        for builder in builders.iter() {
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
                                    current_state.update(msg);
                                }
                                else => { break }
                            }
                        }
                    })
                };

                (builder.incoming_channel(), outgoing, output, current_state, handle)
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

        let _ = sender.send(BuilderIncomingMessages::RequestBuild).await;
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
        Builder, BuilderIncomingMessages, BuilderOutgoingMessages, HashedFileRecord,
    };

    struct TestBuilder;

    impl Builder for TestBuilder {
        fn target(&self) -> Target {
            Target::Android
        }

        fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages> {
            mpsc::channel(1).0
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Utf8PathBuf {
            Utf8PathBuf::from("root_lib")
        }
    }

    struct TestBuilder2;

    impl Builder for TestBuilder2 {
        fn target(&self) -> Target {
            Target::IOS
        }

        fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages> {
            mpsc::channel(1).0
        }

        fn outgoing_channel(
            &self,
        ) -> (
            tokio::sync::broadcast::Receiver<BuilderOutgoingMessages>,
            tokio::sync::broadcast::Receiver<BuildOutputMessages>,
        ) {
            (broadcast::channel(1).1, broadcast::channel(1).1)
        }

        fn root_lib_name(&self) -> Utf8PathBuf {
            Utf8PathBuf::from("root_lib")
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
        incoming: tokio::sync::mpsc::Sender<BuilderIncomingMessages>,
        outgoing: tokio::sync::broadcast::Sender<BuilderOutgoingMessages>,
        output: tokio::sync::broadcast::Sender<BuildOutputMessages>,
        #[allow(dead_code)]
        handle: tokio::task::JoinHandle<()>,
    }

    impl TestChanneledBuilder {
        fn new(target: Target) -> Self {
            let (incoming, mut incoming_rx) = tokio::sync::mpsc::channel(1);
            let (outgoing_tx, _) = tokio::sync::broadcast::channel(1);
            let (output_tx, _) = tokio::sync::broadcast::channel(1);
            let _my_target = target;

            let handle = {
                let outgoing_tx = outgoing_tx.clone();
                let output_tx = output_tx.clone();
                tokio::spawn(async move {
                    while let Some(recv) = incoming_rx.recv().await {
                        match recv {
                            BuilderIncomingMessages::RequestBuild => {
                                if outgoing_tx
                                    .send(BuilderOutgoingMessages::BuildStarted)
                                    .is_err()
                                {
                                    break;
                                }
                                if output_tx
                                    .send(BuildOutputMessages::LibraryUpdated(
                                        HashedFileRecord::new(
                                            "root_lib_path",
                                            Utf8PathBuf::new(),
                                            [
                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                            ],
                                        ),
                                    ))
                                    .is_err()
                                {
                                    break;
                                }
                            }
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

        fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages> {
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

        fn root_lib_name(&self) -> Utf8PathBuf {
            Utf8PathBuf::from("root_lib")
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

            assert_eq!(current_state.root_library, Utf8PathBuf::from("root_lib"));
            assert!(current_state
                .libraries
                .get(&Utf8PathBuf::from("root_lib_path"))
                .is_none());

            let _ = channel.send(BuilderIncomingMessages::RequestBuild).await;

            let message = rx.recv().await.unwrap();
            match message {
                BuildOutputMessages::LibraryUpdated(HashedFileRecord {
                    relative_path,
                    local_path: _,
                    hash,
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

            assert_eq!(current_state.root_library, Utf8PathBuf::from("root_lib"));

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
}
