mod hot_reload_internal;
mod reload_systems;
mod reloadable_app;
mod reloadable_app_setup;
mod replacable_types;
mod schedules;

use bevy::app::Last;
use bevy::ecs::prelude::*;

use bevy::prelude::{App, First, Plugin, PreStartup, Update};

use dexterous_developer_internal::internal::HotReloadInfo;

pub extern crate libloading;

use crate::hot_internal::hot_reload_internal::draw_internal_hot_reload;
use crate::hot_internal::reload_systems::{
    reset_update_frame, toggle_reload_mode, toggle_reloadable_elements, InternalHotReload,
};
pub use crate::types::*;

#[allow(unused_imports)]
pub use reloadable_app_setup::*;

use reload_systems::{cleanup_schedules, reload};
pub use reloadable_app::{ReloadableAppCleanupData, ReloadableAppContents, ReloadableAppElements};
use replacable_types::{ReplacableComponentStore, ReplacableResourceStore};
use schedules::*;

pub struct HotReloadPlugin(HotReloadInfo);

impl HotReloadPlugin {
    pub fn new(info: HotReloadInfo) -> Self {
        println!("Building Hot Reload Plugin");
        Self(info)
    }
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        println!(
            "Build Hot Reload Plugin Thread: {:?}",
            std::thread::current().id()
        );
        let reload_schedule = Schedule::new(SetupReload);
        let cleanup_reloaded_schedule = Schedule::new(CleanupReloaded);
        let cleanup_schedules_schedule = Schedule::new(CleanupSchedules);
        let serialize_schedule = Schedule::new(SerializeReloadables);
        let deserialize_schedule = Schedule::new(DeserializeReloadables);
        let reload_complete = Schedule::new(OnReloadComplete);

        println!("Schedules ready");

        let hot_reload = self.0;

        println!("Set up internal hot reload resources");

        app.add_schedule(reload_schedule)
            .add_schedule(cleanup_reloaded_schedule)
            .add_schedule(cleanup_schedules_schedule)
            .add_schedule(serialize_schedule)
            .add_schedule(deserialize_schedule)
            .add_schedule(reload_complete);

        println!("scheduled attached");

        app.init_resource::<ReloadableAppElements>()
            .init_resource::<ReloadableAppCleanupData>()
            .init_resource::<ReplacableResourceStore>()
            .init_resource::<ReplacableComponentStore>()
            .insert_resource(InternalHotReload(hot_reload, chrono::Local::now(), false));
        println!("Added resources to app");

        app.add_systems(PreStartup, reload)
            .add_systems(CleanupSchedules, cleanup_schedules)
            .add_systems(First, reload)
            .add_systems(Last, reset_update_frame)
            .add_systems(
                Update,
                (
                    draw_internal_hot_reload,
                    toggle_reload_mode,
                    toggle_reloadable_elements,
                ),
            );
        println!("Finished build");
    }
}
