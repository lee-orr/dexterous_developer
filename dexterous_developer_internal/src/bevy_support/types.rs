use super::ReloadableAppContents;
use bevy::{app::PluginGroupBuilder, ecs::schedule::ScheduleLabel, prelude::*};
use serde::{de::DeserializeOwned, Serialize};
pub trait ReplacableResource: Resource + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;
}

pub trait ReplacableComponent: Component + Serialize + DeserializeOwned + Default {
    fn get_type_name() -> &'static str;
}

pub(crate) mod private {
    pub trait ReloadableAppSealed {}
}

pub trait ReloadableApp: private::ReloadableAppSealed {
    fn add_systems<M, L: ScheduleLabel + Eq + ::std::hash::Hash + Clone>(
        &mut self,
        schedule: L,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self;

    fn insert_replacable_resource<R: ReplacableResource>(&mut self) -> &mut Self;
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
        T::generate_reloadable_initializer()
    }
}

impl<P: Plugin> InitialPlugins for P {
    fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder {
        T::generate_reloadable_initializer().add(self)
    }
}

pub trait InitialPlugins {
    fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder;
}

#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct ReloadSettings {
    pub display_update_time: bool,
}
