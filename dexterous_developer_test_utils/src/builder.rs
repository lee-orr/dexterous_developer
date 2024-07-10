use std::env::current_exe;

use camino::Utf8PathBuf;
use dexterous_developer_builder::types::{
    BuildOutputMessages, Builder, BuilderIncomingMessages, BuilderOutgoingMessages,
    HashedFileRecord,
};
use dexterous_developer_types::Target;
use tokio::sync::{
    broadcast,
    mpsc::{self, UnboundedReceiver},
};

pub struct TestBuilder {
    target: Target,
    incoming: tokio::sync::mpsc::UnboundedSender<
        dexterous_developer_builder::types::BuilderIncomingMessages,
    >,
    outgoing: (
        tokio::sync::broadcast::Sender<dexterous_developer_builder::types::BuilderOutgoingMessages>,
        tokio::sync::broadcast::Sender<dexterous_developer_builder::types::BuildOutputMessages>,
    ),
    root_lib_name: Option<String>,
}

pub struct TestBuilderComms {
    pub target: Target,
    pub build_id: u32,
    pub examples: Utf8PathBuf,
    pub target_directory: Utf8PathBuf,
    pub incoming_receiver: UnboundedReceiver<BuilderIncomingMessages>,
    pub outgoing_sender: broadcast::Sender<BuilderOutgoingMessages>,
    pub output_sender: broadcast::Sender<BuildOutputMessages>,
}

impl TestBuilderComms {
    pub fn set_new_library(&mut self, example_name: impl ToString) {
        let example_name = example_name.to_string();
        let example = self.target.dynamic_lib_name(&example_name);
        let path = self.examples.join(&example);
        self.build_id += 1;
        let build = self.build_id;
        self.output_sender
            .send(BuildOutputMessages::StartedBuild(build))
            .unwrap();
        self.output_sender
            .send(BuildOutputMessages::EndedBuild {
                id: build,
                libraries: vec![HashedFileRecord {
                    relative_path: Utf8PathBuf::from("./").join(&example),
                    name: example.to_string(),
                    local_path: path,
                    hash: Default::default(),
                    dependencies: vec![],
                }],
                root_library: example.clone(),
            })
            .unwrap();
    }
}

impl TestBuilder {
    pub fn new(root_lib_name: Option<String>, target: Option<Target>) -> (Self, TestBuilderComms) {
        let target = target.unwrap_or(Target::current().unwrap());
        let (incoming, in_rx) = mpsc::unbounded_channel();
        let (outgoing_tx, _) = broadcast::channel(10);
        let (output_tx, _) = broadcast::channel(10);

        let base = Utf8PathBuf::from_path_buf(current_exe().unwrap()).unwrap();
        let target_directory = base.parent().unwrap().parent().unwrap().to_owned();
        let examples: Utf8PathBuf = target_directory.join("examples");

        (
            Self {
                target,
                incoming,
                outgoing: (outgoing_tx.clone(), output_tx.clone()),
                root_lib_name,
            },
            TestBuilderComms {
                target,
                build_id: 0,
                examples,
                incoming_receiver: in_rx,
                outgoing_sender: outgoing_tx,
                output_sender: output_tx,
                target_directory,
            },
        )
    }
}

impl Builder for TestBuilder {
    fn target(&self) -> dexterous_developer_types::Target {
        self.target
    }

    fn incoming_channel(
        &self,
    ) -> tokio::sync::mpsc::UnboundedSender<
        dexterous_developer_builder::types::BuilderIncomingMessages,
    > {
        self.incoming.clone()
    }

    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<
            dexterous_developer_builder::types::BuilderOutgoingMessages,
        >,
        tokio::sync::broadcast::Receiver<dexterous_developer_builder::types::BuildOutputMessages>,
    ) {
        (self.outgoing.0.subscribe(), self.outgoing.1.subscribe())
    }

    fn root_lib_name(&self) -> Option<String> {
        self.root_lib_name.as_ref().cloned()
    }

    fn get_code_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        vec![]
    }

    fn get_asset_subscriptions(&self) -> Vec<camino::Utf8PathBuf> {
        vec![]
    }
}
