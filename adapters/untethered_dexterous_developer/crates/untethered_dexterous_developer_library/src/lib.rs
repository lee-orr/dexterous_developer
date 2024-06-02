pub mod macros;

use serde::{de::DeserializeOwned, Serialize};

#[derive(Default)]
pub struct ReloadableMain<SerializableState: Serialize + DeserializeOwned + Default, SharedState: Default> {
    serializable: SerializableState,
    shared: SharedState
}

impl<SerializableState: Serialize + DeserializeOwned + Default, SharedState: Default> ReloadableMain<SerializableState, SharedState> {
    
}
