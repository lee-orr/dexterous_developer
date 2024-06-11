use std::borrow::BorrowMut;
use anyhow::bail;
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem};

pub struct ReloadableState<FixedState, Serializable: Serialize + DeserializeOwned> {
    pub fixed: FixedState,
    pub serializable: Box<Serializable>,
}


impl<FixedState, Serializable: Serialize + DeserializeOwned> ReloadableState<FixedState, Serializable> {
    pub fn fixed(&mut self) -> &mut FixedState {
        &mut self.fixed
    }

    pub fn serializable(&mut self) -> &mut Serializable {
        self.serializable.as_mut()
    }

    pub fn mutate(&mut self) -> (&mut FixedState, &mut Serializable) {
        (&mut self.fixed, self.serializable.as_mut())
    }

    pub fn replace_serializable(&mut self, serializable: Serializable) {
        self.serializable = Box::new(serializable);
    }
}


pub trait ReloadableAppLogic {
    type State;

    fn function_name() -> &'static str;

    fn serialization_function_name() -> &'static str;
    fn deserialization_function_name() -> &'static str;

    fn call(state: &mut Self::State) -> Box<AnyWidgetView<Self::State>>;
    fn serialize(state: &mut Self::State) -> anyhow::Result<safer_ffi::Vec<u8>>;
    fn deserialize_into_state(values: safer_ffi::Vec<u8>, state: &mut Self::State) -> anyhow::Result<()>; // TODO - HOW TO HANDLE THIS
}

fn run_reloadable_logic<Logic: ReloadableAppLogic<State = ReloadableState<FixedState, Serializable>>, FixedState, Serializable: Serialize + DeserializeOwned>(
    mut state: &mut ReloadableState<FixedState, Serializable>,
) -> Box<AnyWidgetView<ReloadableState<FixedState, Serializable>>> {
    let info = HOT_RELOAD_INFO.get().expect("Can't access reload info") ;
    if info.update_ready() {
        let serialized = info.call_return::<&mut ReloadableState<FixedState, Serializable>, safer_ffi::Vec<u8>>(Logic::serialization_function_name(), &mut state).unwrap();
        info.update();
        todo!()
    }
    todo!()
}

pub trait XilemReloadableApp<Serializabe: Serialize + DeserializeOwned, FixedState> {
    fn reloadable<Logic: 'static + ReloadableAppLogic<State = ReloadableState<FixedState, Serializabe>>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self;
}


impl<Serializabe: Serialize + DeserializeOwned + 'static, FixedState: 'static>
    XilemReloadableApp<Serializabe, FixedState>
    for Xilem<
        ReloadableState<FixedState, Serializabe>,
        fn(
            &mut ReloadableState<FixedState, Serializabe>,
        ) -> Box<AnyWidgetView<ReloadableState<FixedState, Serializabe>>>,
        Box<AnyWidgetView<ReloadableState<FixedState, Serializabe>>>,
    >
{
    fn reloadable<Logic: 'static + ReloadableAppLogic<State = ReloadableState<FixedState, Serializabe>>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self {
        let state = ReloadableState {
            serializable: Box::new(initial_serializable_state),
            fixed: initial_fixed_state
        };

        let logic = run_reloadable_logic::<Logic, _, _>;
        Xilem::new(state, logic)
    }
}
