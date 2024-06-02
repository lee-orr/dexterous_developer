pub mod macros;

use std::marker::PhantomData;

use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub struct ReloadableMain<
    Events,
    SerializableState: Serialize + DeserializeOwned + Default,
    SharedState: Default,
> {
    pub serializable: SerializableState,
    pub shared: SharedState,
    pub sender: UnboundedSender<Events>,
    pub receiver: UnboundedReceiver<Events>,
    phantom: PhantomData<Events>,
}

impl<Events, SerializableState: Serialize + DeserializeOwned + Default, SharedState: Default>
    Default for ReloadableMain<Events, SerializableState, SharedState>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<Events, SerializableState: Serialize + DeserializeOwned + Default, SharedState: Default>
    ReloadableMain<Events, SerializableState, SharedState>
{
    pub fn new() -> Self {
        let (sender, receiver) = unbounded_channel();

        Self {
            serializable: Default::default(),
            shared: Default::default(),
            sender,
            receiver,
            phantom: PhantomData,
        }
    }

    pub async fn run_loop() {}
}
