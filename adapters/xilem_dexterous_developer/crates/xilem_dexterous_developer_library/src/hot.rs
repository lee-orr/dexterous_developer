use abi_stable::{std_types::RBox, DynTrait, StableAbi};
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(impl_InterfaceType(
    Sync,
    Send
))]
pub struct ReloadableStateInterface;

pub struct ReloadableState {
    serializable: DynTrait<'static, RBox<()>, ReloadableStateInterface>,
    initialized: bool,
}

pub trait ReloadableAppLogic {
    type Serializable;
    type State;

    fn function_name() -> &'static str;

    fn serialization_function_name() -> &'static str;
    fn deserialization_function_name() -> &'static str;

    fn call(state: &mut Self::State) -> Box<AnyWidgetView<Self::State>>;
}

fn run_reloadable_logic<
    Logic: ReloadableAppLogic<State = ReloadableState, Serializable = SerializableState>,
    SerializableState: 'static + Send + Sync + Serialize + DeserializeOwned,
>(
    mut state: &mut ReloadableState,
) -> Box<AnyWidgetView<ReloadableState>> {
    let info = HOT_RELOAD_INFO.get().expect("Can't access reload info");
    if info.update_ready() {
        println!("I'm here");
        if state.initialized {
            println!("Initialized Already?");
            let serialized = info
                .call_return::<&mut ReloadableState, safer_ffi::Vec<u8>>(
                    Logic::serialization_function_name(),
                    &mut state,
                )
                .unwrap();
            info.update();
            info.call::<(
                safer_ffi::Vec<u8>,
                &mut ReloadableState,
            )>(
                Logic::deserialization_function_name(),
                &mut (serialized, &mut state),
            )
            .unwrap();
            println!("Done Serialization Loop");
        } else {
            println!("Initializing");
            let serialized =
                safer_ffi::Vec::from(rmp_serde::to_vec(state.serializable.downcast_as::<SerializableState>().unwrap()).unwrap());
            info.update();
            info.call::<(
                safer_ffi::Vec<u8>,
                &mut ReloadableState,
            )>(
                Logic::deserialization_function_name(),
                &mut (serialized, &mut state),
            )
            .unwrap();
            println!("Initialized now");
            state.initialized = true;
        }
    }
    println!("Reached Here");
    let result = info.call_return::<&mut ReloadableState, Box<AnyWidgetView<ReloadableState>>>(Logic::function_name(), &mut state).unwrap();
    println!("Ran logic loop");
    result
}

pub trait XilemReloadableApp<SerializabeState: Send + Sync + Serialize + DeserializeOwned> {
    fn reloadable<
        Logic: 'static + ReloadableAppLogic<State = ReloadableState, Serializable = SerializabeState>,
    >(
        initial_serializable_state: SerializabeState
    ) -> Self;
}

impl<SerializabeState: Send + Sync + Serialize + DeserializeOwned + 'static>
    XilemReloadableApp<SerializabeState>
    for Xilem<
        ReloadableState,
        fn(
            &mut ReloadableState
        ) -> Box<AnyWidgetView<ReloadableState>>,
        Box<AnyWidgetView<ReloadableState>>,
    >
{
    fn reloadable<
        Logic: 'static + ReloadableAppLogic<State = ReloadableState, Serializable = SerializabeState>,
    >(
        initial_serializable_state: SerializabeState,
    ) -> Self {
        let state = ReloadableState {
            serializable: DynTrait::from_value(initial_serializable_state),
            initialized: false,
        };

        let logic = run_reloadable_logic::<Logic, _>;
        Xilem::new(state, logic)
    }
}
