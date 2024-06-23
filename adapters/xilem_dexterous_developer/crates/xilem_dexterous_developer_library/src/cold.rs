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

pub struct GuardedState<'a, Serializabe: Serialize + DeserializeOwned>(&'a mut Serializabe);

impl<'a, Serializabe: Serialize + DeserializeOwned> Deref for GuardedState<'a, Serializabe> {
    type Target = Serializabe;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, Serializabe: Serialize + DeserializeOwned> DerefMut for GuardedState<'a, Serializabe> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<Serializable: Serialize + DeserializeOwned> ReloadableState<Serializable>
{
    pub fn get(&mut self) ->  GuardedState<Serializable> {
        GuardedState(&mut self.serializable)
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
