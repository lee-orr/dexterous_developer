use dexterous_developer_types::{Target, TargetBuildSettings};

use crate::types::Builder;

pub struct SimpleBuilder {
    target: Target,
    settings: TargetBuildSettings,
    // incoming: tokio::sync::mpsc::Sender<BuilderIncomingMessages>,
    // outgoing: tokio::sync::broadcast::Sender<BuilderOutgoingMessages>,
    // output: tokio::sync::broadcast::Sender<BuildOutputMessages>,
    // #[allow(dead_code)]
    // handle: tokio::task::JoinHandle<()>,
}

impl SimpleBuilder {
    pub fn new(target: Target, settings: TargetBuildSettings) -> Self {
        Self { target, settings }
    }
}

impl Builder for SimpleBuilder {
    fn target(&self) -> Target {
        todo!()
    }

    fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<crate::types::BuilderIncomingMessages> {
        todo!()
    }

    fn outgoing_channel(
        &self,
    ) -> (
        tokio::sync::broadcast::Receiver<crate::types::BuilderOutgoingMessages>,
        tokio::sync::broadcast::Receiver<crate::types::BuildOutputMessages>,
    ) {
        todo!()
    }

    fn root_lib_name(&self) -> Option<camino::Utf8PathBuf> {
        todo!()
    }
}
