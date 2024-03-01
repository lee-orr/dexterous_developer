use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

use dexterous_developer_types::{
    Builder, BuilderIncomingMessages, BuilderOutgoingMessages, HotReloadMessage, Target,
};
use thiserror::Error;
use tokio::sync::{mpsc, watch};

#[derive(Default)]
pub struct Manager {
    targets: HashMap<
        Target,
        (
            mpsc::Sender<BuilderIncomingMessages>,
            watch::Receiver<BuilderOutgoingMessages>,
        ),
    >,
}

#[derive(Error, Debug)]
pub enum ManagerError {
    #[error("Can't build  target {0}")]
    MissingTarget(Target),
    #[error("Can't subscribe to target {0}")]
    SubscriptionFailed(Target),
    #[error("Failed to receive message {0}")]
    ReceiveError(#[from] tokio::sync::watch::error::RecvError),
}

impl Manager {
    pub fn add_builders(mut self, builders: &[Arc<dyn Builder>]) -> Self {
        for builder in builders.iter() {
            for target in builder.targets() {
                self.targets
                    .entry(target)
                    .or_insert_with(|| (builder.incoming_channel(), builder.outgoing_channel()));
            }
        }
        self
    }

    pub fn targets(&self) -> HashSet<Target> {
        self.targets.keys().copied().collect()
    }

    pub async fn watch_target(
        &self,
        target: &Target,
    ) -> Result<watch::Receiver<HotReloadMessage>, ManagerError> {
        let (sender, receiver) = self
            .targets
            .get(target)
            .ok_or(ManagerError::MissingTarget(*target))?;

        let mut rx = receiver.clone();
        let _ = sender
            .send(BuilderIncomingMessages::RequestBuild(*target))
            .await;
        let result = rx
            .wait_for(|value| match value {
                BuilderOutgoingMessages::InvalidTarget(t) => t == target,
                BuilderOutgoingMessages::BuildSubscription(t, _) => t == target,
                _ => false,
            })
            .await?;

        match result.deref() {
            BuilderOutgoingMessages::BuildSubscription(_, recv) => Ok(recv.clone()),
            _ => Err(ManagerError::SubscriptionFailed(*target)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dexterous_developer_types::{BuilderIncomingMessages, BuilderOutgoingMessages};

    struct TestBuilder;

    impl Builder for TestBuilder {
        fn targets(&self) -> Vec<Target> {
            vec![Target::Android]
        }

        fn incoming_channel(
            &self,
        ) -> tokio::sync::mpsc::Sender<dexterous_developer_types::BuilderIncomingMessages> {
            mpsc::channel(1).0
        }

        fn outgoing_channel(
            &self,
        ) -> tokio::sync::watch::Receiver<dexterous_developer_types::BuilderOutgoingMessages>
        {
            watch::channel(BuilderOutgoingMessages::Waiting).1
        }
    }

    struct TestBuilder2;

    impl Builder for TestBuilder2 {
        fn targets(&self) -> Vec<Target> {
            vec![Target::Android, Target::IOS]
        }

        fn incoming_channel(
            &self,
        ) -> tokio::sync::mpsc::Sender<dexterous_developer_types::BuilderIncomingMessages> {
            mpsc::channel(1).0
        }

        fn outgoing_channel(
            &self,
        ) -> tokio::sync::watch::Receiver<dexterous_developer_types::BuilderOutgoingMessages>
        {
            watch::channel(BuilderOutgoingMessages::Waiting).1
        }
    }

    #[test]
    fn when_provided_with_builders_can_return_their_targets() {
        let manager =
            Manager::default().add_builders(&[Arc::new(TestBuilder), Arc::new(TestBuilder2)]);

        let targets = manager.targets();

        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&Target::Android));
        assert!(targets.contains(&Target::IOS));
    }

    #[tokio::test]
    async fn requesting_a_missing_target_returns_an_error() {
        let manager = Manager::default().add_builders(&[Arc::new(TestBuilder)]);

        let err = manager
            .watch_target(&Target::IOS)
            .await
            .expect_err("Didn't fail to watch target");

        assert!(matches!(err, ManagerError::MissingTarget(Target::IOS)));
    }

    struct TestChanneledBuilder {
        target: Target,
        incoming: tokio::sync::mpsc::Sender<BuilderIncomingMessages>,
        outgoing: tokio::sync::watch::Receiver<BuilderOutgoingMessages>,
        #[allow(dead_code)]
        handle: tokio::task::JoinHandle<()>,
    }

    impl TestChanneledBuilder {
        fn new(target: Target) -> Self {
            let (incoming, mut incoming_rx) = tokio::sync::mpsc::channel(1);
            let (outgoing_tx, outgoing) =
                tokio::sync::watch::channel(BuilderOutgoingMessages::Waiting);
            let my_target = target;

            let handle = {
                tokio::spawn(async move {
                    let (target_rx, target_tx) =
                        tokio::sync::watch::channel(HotReloadMessage::KeepAlive);
                    while let Some(recv) = incoming_rx.recv().await {
                        match recv {
                            BuilderIncomingMessages::RequestBuild(target) => {
                                if target == my_target {
                                    if outgoing_tx
                                        .send(BuilderOutgoingMessages::BuildSubscription(
                                            target,
                                            target_tx.clone(),
                                        ))
                                        .is_err()
                                    {
                                        break;
                                    }
                                    if target_rx
                                        .send(HotReloadMessage::RootLibPath(
                                            "root_lib_path".to_string(),
                                        ))
                                        .is_err()
                                    {
                                        break;
                                    }
                                } else if outgoing_tx
                                    .send(BuilderOutgoingMessages::InvalidTarget(target))
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
                outgoing,
                handle,
            }
        }
    }

    impl Builder for TestChanneledBuilder {
        fn targets(&self) -> Vec<Target> {
            vec![self.target]
        }

        fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages> {
            self.incoming.clone()
        }

        fn outgoing_channel(
            &self,
        ) -> tokio::sync::watch::Receiver<dexterous_developer_types::BuilderOutgoingMessages>
        {
            self.outgoing.clone()
        }
    }

    #[tokio::test]
    async fn watching_a_target_returns_a_reciever_for_the_first_matching_builder() {
        let builder_1 = Arc::new(TestChanneledBuilder::new(Target::Android));
        let builder_2 = Arc::new(TestChanneledBuilder::new(Target::Android));

        let channel = builder_1.incoming_channel();

        let manager = Manager::default().add_builders(&[builder_1, builder_2]);

        let rx = manager
            .watch_target(&Target::Android)
            .await
            .expect("Failed to watch target");

        let _ = channel
            .send(BuilderIncomingMessages::RequestBuild(Target::Android))
            .await;

        let recv = rx.borrow();
        assert!(recv.has_changed());
        let message: HotReloadMessage = recv.to_owned();
        match message {
            HotReloadMessage::RootLibPath(message) => assert_eq!(message, "root_lib_path"),
            _ => panic!("Meswsage is wrong type"),
        };
    }
}
