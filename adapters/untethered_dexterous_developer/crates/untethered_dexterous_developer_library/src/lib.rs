pub mod macros;

use serde::{de::DeserializeOwned, Serialize};

#[derive(Default)]
pub struct ReloadableMain<
    SerializableState: Serialize + DeserializeOwned + Default,
    SharedState: Default,
> {
    pub serializable: SerializableState,
    pub shared: SharedState,
}

impl<SerializableState: Serialize + DeserializeOwned + Default, SharedState: Default>
    ReloadableMain<SerializableState, SharedState>
{
}


#[cfg(test)]
mod test {
    #[tokio::test]
    async fn 
}