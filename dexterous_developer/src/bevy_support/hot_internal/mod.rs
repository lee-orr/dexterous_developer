mod hot_reload_internal;
mod reload_systems;
mod reloadable_app;
mod reloadable_app_setup;
mod replacable_types;
mod schedules;

use bevy::ecs::prelude::*;

use bevy::prelude::{App, First, Plugin, PreStartup};

use bevy::utils::Instant;

pub extern crate dexterous_developer_macros;
pub extern crate libloading;

use crate::hot_internal::hot_reload_internal::InternalHotReload;
use crate::internal_shared::lib_path_set::LibPathSet;
pub use crate::types::*;

pub use reloadable_app_setup::*;

use reload_systems::{cleanup, reload, update_lib_system};
pub use reloadable_app::{ReloadableAppCleanupData, ReloadableAppContents};
use replacable_types::{ReplacableComponentStore, ReplacableResourceStore};
use schedules::*;

pub struct HotReloadPlugin(LibPathSet, fn() -> ());

impl HotReloadPlugin {
    pub fn new(libs: String, closure: fn() -> ()) -> Self {
        println!("Building Hot Reload Plugin");
        let libs = libs.clone();
        println!("Lib at path: {libs}");
        Self(LibPathSet::new(libs), closure)
    }
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        println!(
            "Build Hot Reload Plugin Thread: {:?}",
            std::thread::current().id()
        );
        let reload_schedule = Schedule::new();
        let cleanup_schedule = Schedule::new();
        let serialize_schedule = Schedule::new();
        let deserialize_schedule = Schedule::new();
        let reload_complete = Schedule::new();

        println!("Schedules ready");

        let lib_path = self.0.library_path();

        println!("Got lib path");

        let hot_reload = InternalHotReload {
            library: None,
            last_lib: None,
            updated_this_frame: true,
            last_update_time: Instant::now(),
            libs: LibPathSet::new(lib_path),
        };

        println!("Set up internal hot reload resources");

        let watcher = {
            let watch = self.1;
            move || {
                println!("Setting up watcher");
                watch();
            }
        };

        println!("Watcher set up triggered");

        app.add_schedule(SetupReload, reload_schedule)
            .add_schedule(CleanupReloaded, cleanup_schedule)
            .add_schedule(SerializeReloadables, serialize_schedule)
            .add_schedule(DeserializeReloadables, deserialize_schedule)
            .add_schedule(OnReloadComplete, reload_complete);

        println!("scheduled attached");

        app.init_resource::<ReloadableAppContents>()
            .init_resource::<ReloadableAppCleanupData>()
            .init_resource::<ReplacableResourceStore>()
            .init_resource::<ReplacableComponentStore>()
            .insert_resource(hot_reload);
        println!("Added resources to app");

        app.add_systems(PreStartup, (reload, watcher))
            .add_systems(CleanupReloaded, cleanup)
            .add_systems(First, (update_lib_system, reload).chain());
        println!("Finished build");
    }
}
