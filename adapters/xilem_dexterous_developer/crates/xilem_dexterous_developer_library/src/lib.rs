pub mod macros;

use anyhow::bail;
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use serde::{de::DeserializeOwned, Serialize};
use xilem::{core::AnyView, AnyWidgetView, WidgetView, Xilem};

pub struct InternalReloadableState<FixedState> {
    fixed: FixedState,
    serializable: Box<dyn SerailizableState>
}

impl<FixedState> InternalReloadableState<FixedState> {
    pub fn interpret<SerializableState: SerailizableState + DeserializableState>(
        &mut self,
    ) -> anyhow::Result<(&mut FixedState, &mut SerializableState)> {
        let serializable = self.serializable.as_mut();
        let Some(ptr) = (unsafe {
            std::ptr::from_mut(serializable).cast::<SerializableState>().as_mut()
        }) else {
            bail!("Couldn't interpret state");
        };
        Ok((&mut self.fixed, ptr))
    }
}

pub struct ReloadableState<SerializableState: SerailizableState + DeserializableState, FixedState> {
    pub fixed: FixedState,
    pub serializable: Box<SerializableState>,
}

pub trait SerailizableState {
    fn to_bytes(&self) -> Result<Vec<u8>, String>;
}

pub trait DeserializableState: Sized {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String>;
}

impl<S: Serialize> SerailizableState for S {
    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        rmp_serde::to_vec(self).map_err(|e| e.to_string())
    }
}

impl<S: DeserializeOwned> DeserializableState for S {
    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        rmp_serde::from_slice(bytes).map_err(|e| e.to_string())
    }
}

pub trait ReloadableAppLogic<FixedState> {
    fn function_name() -> &'static str;

    fn call_default(
        &self,
        state: &mut InternalReloadableState<FixedState>,
    ) -> Box<AnyWidgetView<InternalReloadableState<FixedState>>>;
}

fn run_reloadable_logic<Logic: ReloadableAppLogic<FixedState>, FixedState>(
    state: &mut InternalReloadableState<FixedState>,
) -> Box<AnyWidgetView<InternalReloadableState<FixedState>>> {
    let info = HOT_RELOAD_INFO.get().expect("Can't access reload info") ;
    if info.update_ready() {
        let bytes = state.serializable.to_bytes().expect("Couldn't serialize state");
        
    }
    todo!()
}

pub trait XilemReloadableApp<Serializabe: SerailizableState + DeserializableState, FixedState> {
    fn reloadable<Logic: 'static + ReloadableAppLogic<FixedState>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self;
}

impl<Serializabe: SerailizableState + DeserializableState + 'static, FixedState: 'static>
    XilemReloadableApp<Serializabe, FixedState>
    for Xilem<
        InternalReloadableState<FixedState>,
        fn(
            &mut InternalReloadableState<FixedState>,
        ) -> Box<AnyWidgetView<InternalReloadableState<FixedState>>>,
        Box<AnyWidgetView<InternalReloadableState<FixedState>>>,
    >
{
    fn reloadable<Logic: 'static + ReloadableAppLogic<FixedState>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self {
        let state = InternalReloadableState {
            serializable: Box::new(initial_serializable_state),
            fixed: initial_fixed_state
        };

        let logic = run_reloadable_logic::<Logic, _>;
        Xilem::new(state, logic)
    }
}
