use std::path::PathBuf;

use bevy::{ecs::schedule::ScheduleLabel, prelude::*, utils::Instant};
use serde::{de::DeserializeOwned, Serialize};

use crate::ReloadableAppContents;

#[derive(Resource, Default, Reflect)]
pub struct HotReload {
    pub last_updated_frame: usize,
    pub version: usize,
    pub updated_this_frame: bool,
}

#[derive(Debug, Event, Reflect)]
pub struct HotReloadEvent {
    pub last_update_time: Instant,
}

#[derive(Debug, Default, Reflect)]
pub struct HotReloadOptions {
    pub lib_name: Option<String>,
    pub watch_folder: Option<PathBuf>,
    pub target_folder: Option<PathBuf>,
}

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct OnReloadComplete;

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
