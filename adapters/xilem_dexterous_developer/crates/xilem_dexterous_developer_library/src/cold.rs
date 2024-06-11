use xilem::{AnyWidgetView, Xilem};

use crate::types::{DeserializableState, SerializableState};

pub trait XilemReloadableApp<Serializabe: SerializableState + DeserializableState, FixedState> {
    fn reloadable<Logic: 'static + ReloadableAppLogic<FixedState, Serializabe>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self;
}

pub trait ReloadableAppLogic<FixedState, Serializable: SerializableState + DeserializableState> {
    type FixedStateType;
    type SerializableStateType;

    fn call_default(
        state: &mut InternalReloadableState<FixedState, Serializable>,
    ) -> Box<AnyWidgetView<InternalReloadableState<FixedState, Serializable>>>;

}

pub struct InternalReloadableState<FixedState, Serializable: SerializableState + DeserializableState> {
    fixed: FixedState,
    serializable: Serializable
}

impl<FixedState, Serializable: SerializableState + DeserializableState> InternalReloadableState<FixedState, Serializable> {
    pub fn interpret(&mut self) -> anyhow::Result<(&mut FixedState, &mut Serializable)> {
        Ok((&mut self.fixed, &mut self.serializable))
    }
}

impl<Serializabe: SerializableState + DeserializableState + 'static, FixedState: 'static>
    XilemReloadableApp<Serializabe, FixedState>
    for Xilem<InternalReloadableState<FixedState, Serializabe>, 
    fn(
        &mut InternalReloadableState<FixedState, Serializabe>,
    ) -> Box<AnyWidgetView<InternalReloadableState<FixedState, Serializabe>>>,
    Box<AnyWidgetView<InternalReloadableState<FixedState, Serializabe>>>> {
        fn reloadable<Logic: 'static + ReloadableAppLogic<FixedState, Serializabe>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self {
        let state = InternalReloadableState {
            serializable: initial_serializable_state,
            fixed: initial_fixed_state
        };

        let logic = Logic::call_default;
        Xilem::new(state, logic)
    }
    }