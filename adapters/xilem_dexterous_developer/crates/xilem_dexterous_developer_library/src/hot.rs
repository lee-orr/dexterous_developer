use std::borrow::BorrowMut;
use anyhow::bail;
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem};

pub struct ReloadableState<FixedState, Serializable: Serialize + DeserializeOwned> {
    fixed: FixedState,
    serializable: Box<Serializable>,
    initialized: bool
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
}

fn run_reloadable_logic<Logic: ReloadableAppLogic<State = ReloadableState<FixedState, Serializable>>, FixedState, Serializable: Serialize + DeserializeOwned>(
    mut state: &mut ReloadableState<FixedState, Serializable>,
) -> Box<AnyWidgetView<ReloadableState<FixedState, Serializable>>> {
    let info = HOT_RELOAD_INFO.get().expect("Can't access reload info") ;
    if info.update_ready() {
        println!("I'm here");
        if state.initialized {
            println!("Initialized Already?");
            let serialized = info.call_return::<&mut ReloadableState<FixedState, Serializable>, safer_ffi::Vec<u8>>(Logic::serialization_function_name(), &mut state).unwrap();
            info.update();
            info.call::<(safer_ffi::Vec<u8>, &mut ReloadableState<FixedState, Serializable>)>(Logic::deserialization_function_name(), &mut (serialized, &mut state)).unwrap();
            println!("Done Serialization Loop");
        } else {
            println!("Initializing");
            info.update();
            println!("Initialized now");
            state.initialized = true;
        }
    }
    println!("Reached Here");
    info.call_return::<&mut ReloadableState<FixedState, Serializable>, Box<AnyWidgetView<ReloadableState<FixedState, Serializable>>>>(Logic::function_name(), &mut state).unwrap()
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
            fixed: initial_fixed_state,
            initialized: false
        };

        let logic = run_reloadable_logic::<Logic, _, _>;
        Xilem::new(state, logic)
    }
}
