mod hot_reload_internal;
mod reload_systems;
mod reloadable_app;
mod reloadable_app_setup;
mod replacable_types;
mod schedules;

use std::time::Duration;

use bevy::ecs::prelude::*;

use bevy::prelude::{App, First, Plugin, PreStartup};

use bevy::utils::Instant;

pub extern crate dexterous_developer_macros;
pub extern crate libloading;

use crate::internal_shared::lib_path_set::LibPathSet;
pub use crate::types::*;

pub use reloadable_app_setup::*;

use crate::hot_internal::hot_reload_internal::InternalHotReload;
use crate::hot_internal::reload_systems::{cleanup, reload, update_lib_system};
pub use crate::hot_internal::reloadable_app::{ReloadableAppCleanupData, ReloadableAppContents};
use crate::hot_internal::replacable_types::{ReplacableComponentStore, ReplacableResourceStore};
use crate::hot_internal::schedules::*;

pub struct HotReloadPlugin(LibPathSet);

impl HotReloadPlugin {
    pub fn new(libs: LibPathSet) -> Self {
        Self(libs)
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

        app.add_schedule(SetupReload, reload_schedule)
            .add_schedule(CleanupReloaded, cleanup_schedule)
            .add_schedule(SerializeReloadables, serialize_schedule)
            .add_schedule(DeserializeReloadables, deserialize_schedule)
            .add_schedule(OnReloadComplete, reload_complete)
            .init_resource::<ReloadableAppContents>()
            .init_resource::<ReloadableAppCleanupData>()
            .init_resource::<ReplacableResourceStore>()
            .init_resource::<ReplacableComponentStore>()
            .insert_resource(InternalHotReload {
                library: None,
                last_lib: None,
                updated_this_frame: true,
                last_update_time: Instant::now().checked_sub(Duration::from_secs(1)).unwrap(),
                libs: self.0.clone(),
            })
            .add_systems(PreStartup, reload)
            .add_systems(CleanupReloaded, cleanup)
            .add_systems(First, (update_lib_system, reload).chain());
        println!("Finished build");
    }
}
