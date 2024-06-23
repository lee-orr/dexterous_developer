use abi_stable::{std_types::RBox, DynTrait, StableAbi};
use dexterous_developer_internal::internal::HOT_RELOAD_INFO;
use serde::Deserialize;
use serde::{de::DeserializeOwned, Serialize};
use xilem::{AnyWidgetView, Xilem, core::View};

#[repr(C)]
#[derive(StableAbi)]
#[sabi(impl_InterfaceType(
    Sync,
    Send,
    Serialize,
    Deserialize,
    Clone
))]
pub struct ReloadableStateInterface;

pub struct ReloadableState {
    serializable: DynTrait<'static, RBox<()>, ReloadableStateInterface>,
    initialized: bool,
}

impl ReloadableState {
    pub fn get_dyn_trait(&mut self) -> &mut DynTrait<'static, RBox<()>, ReloadableStateInterface> {
        &mut self.serializable
    }

    pub fn set_dyn_trait(&mut self, dyntrait: DynTrait<'static, RBox<()>, ReloadableStateInterface>) {
        self.serializable = dyntrait;
    }
}

pub trait SerializableState: Send + Sync + Serialize + DeserializeOwned + Clone {}




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
    SerializableState: 'static + Send + Clone + Sync + Serialize + DeserializeOwned,
>(
    state: &mut Mutex<Option<SerializableState>>,
) -> ReloadingView<SerializableState, Logic> {
    ReloadingView {initial_serializable_state: std::mem::take(state), phantom: std::marker::PhantomData}
}


use std::sync::Mutex;
pub struct ReloadingView<SerializableState: Send + Sync + Clone + Serialize + DeserializeOwned + 'static, Logic: ReloadableAppLogic<State = ReloadableState, Serializable = SerializableState>,>{
    initial_serializable_state: Mutex<Option<SerializableState>>,
    phantom: std::marker::PhantomData<Logic>
}

use xilem::ViewCtx;

struct DynamicViewState {
    current_view: Option<(Box<AnyWidgetView<ReloadableState>>, <Box<AnyWidgetView<ReloadableState>> as View<ReloadableState, (), ViewCtx>>::ViewState)>,
    state: ReloadableState,
}

use xilem::Pod;
use masonry::widget::WidgetMut;

impl<'a, SerializableState: Send + Sync + Clone + Serialize + DeserializeOwned + 'static, Logic: ReloadableAppLogic<State = ReloadableState, Serializable = SerializableState> + 'static,> View<(), (), ViewCtx> for ReloadingView<SerializableState, Logic> {
    type Element = Pod<DynWidget>;
    type ViewState = DynamicViewState;
    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let info = HOT_RELOAD_INFO.get().expect("Can't access reload info");
        let value =self.initial_serializable_state.lock().unwrap().take().unwrap();
        let  mut state = ReloadableState {
            serializable: DynTrait::from_value(value),
            initialized: false, //TODO: yeet
        };
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
        let first_view = info.call_return::<ReloadableState, Box<AnyWidgetView<ReloadableState>>>(Logic::function_name(), &mut state).unwrap();
        let (element, inner_state) = first_view.build(ctx);
        let element = DynWidget {inner: element.inner.boxed()};
        (Pod::new(element), DynamicViewState {current_view: Some((first_view, inner_state)), state })

    }
    fn rebuild<'el>(
            &self,
            _: &Self,
            view_state: &mut Self::ViewState,
            ctx: &mut ViewCtx,
          mut  element: xilem::core::Mut<'el, Self::Element>,
        ) -> xilem::core::Mut<'el, Self::Element> {
            let info = HOT_RELOAD_INFO.get().expect("Can't access reload info");
            let mut state = &mut view_state.state;
            if info.update_ready() {
                println!("Initialized Already?");
                // Call view.teardown on your *old* state and widget tree
                let (old, mut inner_state) = view_state.current_view.take().unwrap();
                let mut x=  element.ctx.get_mut(&mut element.widget.inner);
                let old_element = x.downcast();
                old.teardown(&mut inner_state, ctx, old_element);
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

                // Call view.build on your view tree state and create new widget tree
                let next_view = info.call_return::<ReloadableState, Box<AnyWidgetView<ReloadableState>>>(Logic::function_name(), &mut state).unwrap();
                let (new_element, inner_state) = next_view.build(ctx);
                DynWidget::replace_inner(&mut element, new_element.inner.boxed());
                view_state.current_view = Some((next_view, inner_state));
                
                println!("Done Serialization Loop");
            } else {
                let (old, mut inner_state) = view_state.current_view.take().unwrap();
                // Call view.build on your view tree state and create new widget tree
                let next_view = info.call_return::<ReloadableState, Box<AnyWidgetView<ReloadableState>>>(Logic::function_name(), &mut state).unwrap();
                let mut x=  element.ctx.get_mut(&mut element.widget.inner);
                let not_old_element = x.downcast();
                next_view.rebuild(&old, &mut inner_state, ctx, not_old_element);
                view_state.current_view = Some((next_view, inner_state));
            }
            element
    }

    fn teardown(
            &self,
            view_state: &mut Self::ViewState,
            ctx: &mut ViewCtx,
            element: xilem::core::Mut<'_, Self::Element>,
        ) {
        todo!("Only will be called on app shutdown anyway, we can crash :)");
    }
    fn message(
            &self,
            view_state: &mut Self::ViewState,
            id_path: &[xilem::core::ViewId],
            message: xilem::core::DynMessage,
            app_state: &mut (),
        ) -> xilem::core::MessageResult<()> {
            let x = view_state.current_view.as_mut().unwrap();
            let (inner,  inner_state) =  x;
            
            inner.message(&mut *inner_state, id_path, message, &mut view_state.state)
    }


}

use masonry::{WidgetPod, Widget, EventCtx, PointerEvent, TextEvent, AccessEvent};

/// A widget whose only child can be dynamically replaced.
///
/// `WidgetPod<Box<dyn Widget>>` doesn't expose this possibility.
pub struct DynWidget {
    inner: WidgetPod<Box<dyn Widget>>,
}

impl DynWidget {
    pub(crate) fn replace_inner(
        this: &mut WidgetMut<'_, Self>,
        widget: WidgetPod<Box<dyn Widget>>,
    ) {
        this.widget.inner = widget;
        this.ctx.children_changed();
    }
}

/// Forward all events to the child widget.
impl Widget for DynWidget {
    fn on_pointer_event(&mut self, ctx: &mut EventCtx, event: &PointerEvent) {
        self.inner.on_pointer_event(ctx, event);
    }
    fn on_text_event(&mut self, ctx: &mut EventCtx, event: &TextEvent) {
        self.inner.on_text_event(ctx, event);
    }
    fn on_access_event(&mut self, ctx: &mut EventCtx, event: &AccessEvent) {
        self.inner.on_access_event(ctx, event);
    }

    fn on_status_change(&mut self, _: &mut LifeCycleCtx, _: &StatusChange) {
        // Intentionally do nothing
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle) {
        self.inner.lifecycle(ctx, event);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints) -> Size {
        let size = self.inner.layout(ctx, bc);
        ctx.place_child(&mut self.inner, Point::ORIGIN);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, scene: &mut Scene) {
        self.inner.paint(ctx, scene);
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, ctx: &mut AccessCtx) {
        self.inner.accessibility(ctx);
    }

    fn children(&self) -> SmallVec<[WidgetRef<'_, dyn Widget>; 16]> {
        let mut vec = SmallVec::new();
        vec.push(self.inner.as_dyn());
        vec
    }
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
            &mut Mutex<Option<SerializabeState>>
        ) -> ReloadingView<SerializabeState, Logic>,
        DynWidget,
    >
{
    fn reloadable<
        Logic: 'static + ReloadableAppLogic<State = ReloadableState, Serializable = SerializabeState>,
    >(
        initial_serializable_state: SerializabeState,
    ) -> Self {
        let state = Mutex::new(Some(

            initial_serializable_state
        ))
            ;

        let logic = run_reloadable_logic::<Logic, _>;
        Xilem::new(state, logic)
    }
}
