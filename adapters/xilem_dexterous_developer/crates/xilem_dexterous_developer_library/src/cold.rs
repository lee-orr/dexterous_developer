use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem};

pub trait XilemReloadableApp<Serializabe: Serialize + DeserializeOwned, FixedState> {
    fn reloadable<Logic: 'static + ReloadableAppLogic<State = ReloadableState<FixedState, Serializabe>>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self;
}

pub trait ReloadableAppLogic {
    type State;

    fn call_default(
        state: &mut Self::State,
    ) -> Box<AnyWidgetView<Self::State>>;

}

pub struct ReloadableState<FixedState, Serializable: Serialize + DeserializeOwned> {
    fixed: FixedState,
    serializable: Serializable
}

impl<FixedState, Serializable: Serialize + DeserializeOwned> ReloadableState<FixedState, Serializable> {
    pub fn fixed(&mut self) -> &mut FixedState {
        &mut self.fixed
    }

    pub fn serializable(&mut self) -> &mut Serializable {
        &mut self.serializable
    }

    pub fn mutate(&mut self) -> (&mut FixedState, &mut Serializable) {
        (&mut self.fixed, &mut self.serializable)
    }
}

impl<Serializabe: Serialize + DeserializeOwned + 'static, FixedState: 'static>
    XilemReloadableApp<Serializabe, FixedState>
    for Xilem<ReloadableState<FixedState, Serializabe>, 
    fn(
        &mut ReloadableState<FixedState, Serializabe>,
    ) -> Box<AnyWidgetView<ReloadableState<FixedState, Serializabe>>>,
    Box<AnyWidgetView<ReloadableState<FixedState, Serializabe>>>> {
        fn reloadable<Logic: 'static + ReloadableAppLogic<State = ReloadableState<FixedState, Serializabe>>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self {
        let state = ReloadableState {
            serializable: initial_serializable_state,
            fixed: initial_fixed_state
        };

        let logic = Logic::call_default;
        Xilem::new(state, logic)
    }
    }