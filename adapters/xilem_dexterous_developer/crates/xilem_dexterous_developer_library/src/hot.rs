use anyhow::bail;
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use xilem::{AnyWidgetView, Xilem};

use crate::types::*;

pub struct InternalReloadableState<FixedState> {
    fixed: FixedState,
    serializable: Box<dyn SerializableState>
}

impl<FixedState> InternalReloadableState<FixedState> {
    pub fn interpret<Serializable: SerializableState + DeserializableState>(
        &mut self,
    ) -> anyhow::Result<(&mut FixedState, &mut Serializable)> {
        let serializable = self.serializable.as_mut();
        let Some(ptr) = (unsafe {
            std::ptr::from_mut(serializable).cast::<Serializable>().as_mut()
        }) else {
            bail!("Couldn't interpret state");
        };
        Ok((&mut self.fixed, ptr))
    }
}

pub struct ReloadableState<Serializable: SerializableState + DeserializableState, FixedState> {
    pub fixed: FixedState,
    pub serializable: Box<Serializable>,
}


pub trait ReloadableAppLogic<FixedState> {
    fn function_name() -> &'static str;
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

pub trait XilemReloadableApp<Serializabe: SerializableState + DeserializableState, FixedState> {
    fn reloadable<Logic: 'static + ReloadableAppLogic<FixedState>>(
        initial_serializable_state: Serializabe,
        initial_fixed_state: FixedState,
    ) -> Self;
}

impl<Serializabe: SerializableState + DeserializableState + 'static, FixedState: 'static>
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
