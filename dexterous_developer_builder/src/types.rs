use dexterous_developer_types::{HotReloadMessage, Target};

pub trait Builder: 'static + Send + Sync {
    fn targets(&self) -> Vec<Target>;
    fn incoming_channel(&self) -> tokio::sync::mpsc::Sender<BuilderIncomingMessages>;
    fn outgoing_channel(&self) -> tokio::sync::watch::Receiver<BuilderOutgoingMessages>;
}

#[derive(Debug, Clone)]
pub enum BuilderIncomingMessages {
    RequestBuild(Target),
}

#[derive(Debug, Clone)]
pub enum BuilderOutgoingMessages {
    Waiting,
    InvalidTarget(Target),
    BuildSubscription(Target, tokio::sync::watch::Receiver<HotReloadMessage>),
}
