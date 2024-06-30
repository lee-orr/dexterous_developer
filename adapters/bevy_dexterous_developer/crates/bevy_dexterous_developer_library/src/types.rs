use super::ReloadableAppContents;
use bevy::{app::PluginGroupBuilder, ecs::schedule::ScheduleLabel, log::LogPlugin, prelude::*};
use serde::{de::DeserializeOwned, Serialize};

pub trait ReplacableResource: Resource + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;
}

pub trait CustomReplacableResource: Resource + Sized {
    fn get_type_name() -> &'static str;

    fn to_vec(&self) -> anyhow::Result<Vec<u8>>;

    fn from_slice(val: &[u8]) -> anyhow::Result<Self>;
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
}

pub trait ReplacableComponent: Component + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;
}
pub trait ReplacableEvent: Event + Serialize + DeserializeOwned {
    fn get_type_name() -> &'static str;
}

pub trait ReplacableState: States + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;
    fn get_next_type_name() -> &'static str;
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
}
pub(crate) mod private {
    pub trait ReloadableAppSealed {}
}

pub trait ReloadableApp: private::ReloadableAppSealed + AppExtStates {
    fn add_systems<M, L: ScheduleLabel + Eq + ::std::hash::Hash + Clone>(
        &mut self,
        schedule: L,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;

    fn init_replacable_resource<R: CustomReplacableResource + Default>(&mut self) -> &mut Self;
    fn insert_replacable_resource<R: CustomReplacableResource>(
        &mut self,
        initializer: impl 'static + Send + Sync + Fn() -> R,
    ) -> &mut Self;
    fn reset_resource<R: Resource + Default>(&mut self) -> &mut Self;
    fn reset_resource_to_value<R: Resource + Clone>(&mut self, value: R) -> &mut Self;
    fn register_replacable_component<C: ReplacableComponent>(&mut self) -> &mut Self;
    fn clear_marked_on_reload<C: Component>(&mut self) -> &mut Self;
    fn reset_setup<C: Component, M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self;
    fn reset_setup_in_state<C: Component, S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;
    fn add_event<T: ReplacableEvent>(&mut self) -> &mut Self;
}

pub trait ReloadableSetup {
    fn setup_function_name() -> &'static str;
    fn default_function(app: &mut ReloadableAppContents);
}

pub trait ReloadableElementsSetup {
    fn setup_reloadable_elements<T: ReloadableSetup>(&mut self) -> &mut Self;
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
    fn generate_reloadable_initializer() -> PluginGroupBuilder;
}

impl InitializablePlugins for DefaultPlugins {
    fn generate_reloadable_initializer() -> PluginGroupBuilder {
        get_default_plugins()
    }
}
impl InitializablePlugins for MinimalPlugins {
    fn generate_reloadable_initializer() -> PluginGroupBuilder {
        get_minimal_plugins()
    }
}

pub struct InitialPluginsEmpty;

impl InitialPlugins for InitialPluginsEmpty {
    fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder {
        let initializer = T::generate_reloadable_initializer();
        if tracing::dispatcher::has_been_set() {
            initializer.disable::<LogPlugin>()
        } else {
            initializer
        }
    }
}

impl<P: Plugin> InitialPlugins for P {
    fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder {
        let initializer = T::generate_reloadable_initializer().add(self);
        if tracing::dispatcher::has_been_set() {
            initializer.disable::<LogPlugin>()
        } else {
            initializer
        }
    }
}

pub trait InitialPlugins {
    fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder;
}

/// These are dynamically adjustable settings for reloading. Ignored when not hot reloading.
#[derive(Resource, Clone, Debug)]
pub struct ReloadSettings {
    /// Toggles whether the last update time is displayed in the window title. Only applicable when `reload_mode` is not `ReloadMode::Full`.
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
