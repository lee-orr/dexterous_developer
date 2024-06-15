use std::ops::{Deref, DerefMut};

use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem};

pub trait XilemReloadableApp<Serializabe: Serialize + DeserializeOwned> {
    fn reloadable<
        Logic: 'static + ReloadableAppLogic<State = ReloadableState<Serializabe>>,
    >(
        initial_serializable_state: Serializabe
    ) -> Self;
}

pub trait ReloadableAppLogic {
    type State;

    fn call_default(state: &mut Self::State) -> Box<AnyWidgetView<Self::State>>;
}

pub struct ReloadableState<Serializable: Serialize + DeserializeOwned> {
    serializable: Serializable,
}

impl<Serializable: Serialize + DeserializeOwned> AsMut<Serializable> for
    ReloadableState<Serializable>
{
    fn as_mut(&mut self) -> &mut Serializable {
        &mut self.serializable
    }
}

impl<Serializable: Serialize + DeserializeOwned + Sized> DerefMut for
    ReloadableState<Serializable>
{
    fn deref_mut(&mut self) -> &mut Serializable {
        &mut self.serializable
    }
}
impl<Serializable: Serialize + DeserializeOwned> Deref for
    ReloadableState<Serializable>
{
    type Target = Serializable;

    fn deref(&self) -> &Self::Target {
        &self.serializable
    }
}

impl<Serializabe: Serialize + DeserializeOwned + 'static>
    XilemReloadableApp<Serializabe>
    for Xilem<
        ReloadableState<Serializabe>,
        fn(
            &mut ReloadableState<Serializabe>,
        ) -> Box<AnyWidgetView<ReloadableState<Serializabe>>>,
        Box<AnyWidgetView<ReloadableState<Serializabe>>>,
    >
{
    fn reloadable<
        Logic: 'static + ReloadableAppLogic<State = ReloadableState<Serializabe>>,
    >(
        initial_serializable_state: Serializabe,
    ) -> Self {
        let state = ReloadableState {
            serializable: initial_serializable_state,
        };

        let logic = Logic::call_default;
        Xilem::new(state, logic)
    }
}
