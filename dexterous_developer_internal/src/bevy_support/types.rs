use std::marker::PhantomData;

use bevy::{app::PluginGroupBuilder, prelude::*};
use serde::{de::DeserializeOwned, Serialize};

pub trait ReloadableElementLabel: 'static + std::hash::Hash {
    fn get_element_name() -> &'static str;
}

impl ReloadableElementLabel for () {
    fn get_element_name() -> &'static str {
        "default_reloadable_element"
    }
}

pub trait AttachReloadableElementLabel<T: ReloadableElementLabel> {}

pub trait GetElementLabel<M> {
    fn get_element_label() -> &'static str;
}

impl<T: ReloadableElementLabel, R: AttachReloadableElementLabel<T>> GetElementLabel<(T, R)> for R {
    fn get_element_label() -> &'static str {
        T::get_element_name()
    }
}

pub trait ReplacableResource<T: ReloadableElementLabel = ()>:
    Resource + Serialize + DeserializeOwned + Default
{
    fn get_type_name() -> &'static str;

    fn get_element_label() -> &'static str {
        <() as ReloadableElementLabel>::get_element_name()
    }
}

pub trait CustomReplacableResource: Resource + Default {
    fn get_type_name() -> &'static str;

    fn to_vec(&self) -> anyhow::Result<Vec<u8>>;

    fn from_slice(val: &[u8]) -> anyhow::Result<Self>;

    fn get_element_label() -> &'static str {
        <() as ReloadableElementLabel>::get_element_name()
    }
}

impl<T: CustomReplacableResource> GetElementLabel<(T, ())> for T {
    fn get_element_label() -> &'static str {
        T::get_element_label()
    }
}

impl<T: ReplacableResource> CustomReplacableResource for T {
    fn get_type_name() -> &'static str {
        T::get_type_name()
    }

    fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        Ok(rmp_serde::to_vec(self)?)
    }

    fn from_slice(val: &[u8]) -> anyhow::Result<Self> {
        Ok(rmp_serde::from_slice(val)?)
    }

    fn get_element_label() -> &'static str {
        T::get_element_label()
    }
}

pub trait ReplacableComponent: Component + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;

    fn get_element_label() -> &'static str {
        <() as ReloadableElementLabel>::get_element_name()
    }
}

impl<T: ReplacableComponent> GetElementLabel<(T, (), ())> for T {
    fn get_element_label() -> &'static str {
        T::get_element_label()
    }
}
pub trait ReplacableEvent: Event + Serialize + DeserializeOwned {
    fn get_type_name() -> &'static str;

    fn get_element_label() -> &'static str {
        <() as ReloadableElementLabel>::get_element_name()
    }
}

impl<T: ReplacableEvent> GetElementLabel<(T, (), T)> for T {
    fn get_element_label() -> &'static str {
        T::get_element_label()
    }
}

pub trait ReplacableState: States + Serialize + DeserializeOwned {
    fn get_type_name() -> &'static str;
    fn get_next_type_name() -> &'static str;

    fn get_element_label() -> &'static str {
        <() as ReloadableElementLabel>::get_element_name()
    }
}

impl<T: ReplacableState> GetElementLabel<(T, T, ())> for T {
    fn get_element_label() -> &'static str {
        T::get_element_label()
    }
}

impl<S: ReplacableState> CustomReplacableResource for State<S> {
    fn get_type_name() -> &'static str {
        S::get_type_name()
    }

    fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        Ok(rmp_serde::to_vec(self.get())?)
    }

    fn from_slice(val: &[u8]) -> anyhow::Result<Self> {
        let val = rmp_serde::from_slice(val)?;
        Ok(Self::new(val))
    }

    fn get_element_label() -> &'static str {
        S::get_element_label()
    }
}

impl<S: ReplacableState> CustomReplacableResource for NextState<S> {
    fn get_type_name() -> &'static str {
        S::get_next_type_name()
    }

    fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        Ok(rmp_serde::to_vec(&self.0)?)
    }

    fn from_slice(val: &[u8]) -> anyhow::Result<Self> {
        let val = rmp_serde::from_slice(val)?;
        Ok(Self(val))
    }

    fn get_element_label() -> &'static str {
        S::get_element_label()
    }
}

impl<S: ReplacableEvent> CustomReplacableResource for Events<S> {
    fn get_type_name() -> &'static str {
        S::get_type_name()
    }

    fn to_vec(&self) -> anyhow::Result<Vec<u8>> {
        Ok(vec![])
    }

    fn from_slice(_: &[u8]) -> anyhow::Result<Self> {
        Ok(Self::default())
    }

    fn get_element_label() -> &'static str {
        S::get_element_label()
    }
}
pub(crate) mod private {
    pub trait ReloadableAppSealed {}
}

pub trait ReloadableApp: private::ReloadableAppSealed {
    fn register_replacable_resource<R: CustomReplacableResource>(&mut self) -> &mut Self;
    fn reset_resource<R: Resource + Default + GetElementLabel<L>, L>(&mut self) -> &mut Self;
    fn reset_resource_to_value<R: Resource + Clone + GetElementLabel<L>, L>(
        &mut self,
        value: R,
    ) -> &mut Self;
    fn register_replacable_component<C: ReplacableComponent>(&mut self) -> &mut Self;
    fn clear_marked_on_reload<C: Component + GetElementLabel<L>, L>(&mut self) -> &mut Self;
    fn reset_setup<C: Component + GetElementLabel<L>, M, L>(
        &mut self,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;
    fn reset_setup_in_state<C: Component + GetElementLabel<L>, S: States, M, L>(
        &mut self,
        state: S,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;
    fn add_reloadable_event<T: ReplacableEvent>(&mut self) -> &mut Self;
    fn add_reloadable_state<S: ReplacableState>(&mut self) -> &mut Self;
}

pub fn clear_marked_system<C: Component>(mut commands: Commands, q: Query<Entity, With<C>>) {
    for entity in q.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
pub fn get_default_plugins() -> PluginGroupBuilder {
    DefaultPlugins.build()
}

pub fn get_minimal_plugins() -> PluginGroupBuilder {
    MinimalPlugins.build()
}

pub trait InitializablePlugins: PluginGroup {
    fn initialize_fence() -> PluginGroupBuilder;

    fn initialize_app() -> PluginGroupBuilder;

    fn initialize_hot_app() -> PluginGroupBuilder;
}

impl InitializablePlugins for DefaultPlugins {
    fn initialize_fence() -> PluginGroupBuilder {
        get_default_plugins()
    }

    fn initialize_app() -> PluginGroupBuilder {
        get_default_plugins()
    }

    fn initialize_hot_app() -> PluginGroupBuilder {
        get_default_plugins()
    }
}
impl InitializablePlugins for MinimalPlugins {
    fn initialize_fence() -> PluginGroupBuilder {
        get_minimal_plugins()
    }

    fn initialize_app() -> PluginGroupBuilder {
        get_minimal_plugins()
    }

    fn initialize_hot_app() -> PluginGroupBuilder {
        get_minimal_plugins()
    }
}

pub struct InitialPluginsEmpty<'a>(&'a mut App);

impl<'a> InitialPluginsEmpty<'a> {
    pub fn new(app: &'a mut App) -> Self {
        Self(app)
    }
}

pub trait InitializeApp<'a> {
    type PluginsReady<T: InitializablePlugins>: PluginsReady<'a, T>;

    fn initialize<T: InitializablePlugins>(self) -> Self::PluginsReady<T>;
}

impl<'a> InitializeApp<'a> for InitialPluginsEmpty<'a> {
    type PluginsReady<T: InitializablePlugins> = InitialPluginsReady<'a, T>;

    fn initialize<T: InitializablePlugins>(self) -> Self::PluginsReady<T> {
        let group = T::initialize_fence();
        InitialPluginsReady::<T>(group, self.0, vec![], PhantomData)
    }
}

pub struct InitialPluginsReady<'a, T: InitializablePlugins>(
    PluginGroupBuilder,
    &'a mut App,
    Vec<Box<dyn FnOnce(&mut App)>>,
    PhantomData<T>,
);

impl<'a, T: InitializablePlugins> PluginsReady<'a, T> for InitialPluginsReady<'a, T> {
    fn adjust<F: Fn(PluginGroupBuilder) -> PluginGroupBuilder>(mut self, adjust_fn: F) -> Self {
        self.0 = adjust_fn(self.0);
        self
    }

    fn app(self) -> &'a mut App {
        let app = self.1;
        app.add_plugins(self.0);
        for mod_fn in self.2.into_iter() {
            mod_fn(app);
        }
        app
    }

    fn modify_fence<F: 'static + FnOnce(&mut App)>(mut self, fence_fn: F) -> Self {
        self.2.push(Box::new(fence_fn));
        self
    }
}

pub trait SetPluginRunner<'a> {
    fn app_with_runner<T: 'static + FnOnce(App) + Send>(self, runner: T) -> &'a mut App;
}

impl<'a, P: PluginsReady<'a, MinimalPlugins>> SetPluginRunner<'a> for P {
    fn app_with_runner<T: 'static + FnOnce(App) + Send>(self, runner: T) -> &'a mut App {
        self.modify_fence(move |app| {
            app.set_runner(runner);
        })
        .app()
    }
}

pub trait PluginsReady<'a, T: InitializablePlugins>: Sized {
    fn adjust<F: Fn(PluginGroupBuilder) -> PluginGroupBuilder>(self, adjust_fn: F) -> Self;

    fn modify_fence<F: 'static + FnOnce(&mut App)>(self, fence_fn: F) -> Self;

    fn app(self) -> &'a mut App;

    fn sync_resource_from_fence<R: Resource + Clone>(self) -> Self {
        self
    }

    fn sync_resource_from_app<R: Resource + Clone>(self) -> Self {
        self
    }

    fn sync_resource_bi_directional<R: Resource + Clone>(self) -> Self {
        self
    }
}
/// These are dynamically adjustable settings for reloading. Ignored when not hot reloading.
#[derive(Resource, Clone, Debug)]
pub struct ReloadSettings {
    /// Toggles whether the last update time is displayed in the window title. Only applicable when reload_mode is not "Full".
    pub display_update_time: bool,
    /// Sets the reload mode
    pub reload_mode: ReloadMode,
    /// Sets a key for manually triggering a reload cycle. Depending on the reload mode, it will re-set the schedules, serialize/deserialize reloadables, and re run any cleanup or setup functions.
    pub manual_reload: Option<KeyCode>,
    /// Sets a key to manually cycle between reload modes in order - Full, System and Setup, System Only
    pub toggle_reload_mode: Option<KeyCode>,
    /// Enable the capacity to cycle between reloading different reloadable element functions.
    pub reloadable_element_policy: ReloadableElementPolicy,
    /// The current selected reloadable element
    pub reloadable_element_selection: Option<&'static str>,
}

impl Default for ReloadSettings {
    fn default() -> Self {
        Self {
            display_update_time: true,
            manual_reload: Some(KeyCode::F2),
            toggle_reload_mode: Some(KeyCode::F1),
            reload_mode: ReloadMode::Full,
            reloadable_element_policy: ReloadableElementPolicy::OneOfAll(KeyCode::F3),
            reloadable_element_selection: None,
        }
    }
}

/// These are the different modes for hot-reloading
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ReloadableElementPolicy {
    /// Reloads All Reloadable Elements
    #[default]
    All,
    /// Allows cycling among all the available reloadable elements using the provided key
    OneOfAll(KeyCode),
    /// Allows cycling among a limited set of the reloadable elements using the provided key
    OneOfList(KeyCode, Vec<&'static str>),
}

/// These are the different modes for hot-reloading
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReloadMode {
    /// This reloads systems/schedules, serializes/deserializes reloadable resources and components, and runs cleanup & setup functions.
    #[default]
    Full,
    /// This reloads systems/schedules and runs cleanup and setup functions, but does not serialize/deserialize resources or components.
    SystemAndSetup,
    /// This only reloads systems and schedules, and does not run any cleanup or setup functions.
    SystemOnly,
}

impl ReloadMode {
    pub fn should_serialize(&self) -> bool {
        *self == Self::Full
    }

    pub fn should_run_setups(&self) -> bool {
        *self == Self::Full || *self == Self::SystemAndSetup
    }
}
