mod hot_reload_internal;
mod lib_set;
mod library_holder;
mod reload_systems;
mod reloadable_app;
mod reloadable_app_setup;
mod replacable_types;
mod schedules;
mod update_lib;
mod watch;

use std::time::Duration;

use bevy::ecs::prelude::*;

use bevy::prelude::{App, First, Plugin, PreStartup};

use bevy::utils::Instant;

pub extern crate libloading;
pub extern crate reload_macros;

pub use crate::types::*;
use lib_set::*;

pub use reloadable_app_setup::*;

use crate::hot::hot_reload_internal::InternalHotReload;
use crate::hot::reload_systems::{cleanup, reload, update_lib_system};
pub use crate::hot::reloadable_app::{ReloadableAppCleanupData, ReloadableAppContents};
use crate::hot::replacable_types::{ReplacableComponentStore, ReplacableResourceStore};
use crate::hot::schedules::*;
use crate::hot::update_lib::get_initial_library;
use crate::hot::watch::{run_watcher, EndWatch};

pub fn run_reloadabe_app(options: HotReloadOptions) {
    println!("Current Thread: {:?}", std::thread::current().id());
    let library_paths = LibPathSet::new(&options).unwrap();
    println!("Paths: {library_paths:?}");

    let _ = std::fs::remove_file(library_paths.library_path());

    let (end_watch_tx, end_watch_rx) = crossbeam::channel::bounded::<EndWatch>(1);

    run_watcher(end_watch_rx.clone(), &library_paths);

    let lib = get_initial_library(&library_paths);

    if let Some(lib) = lib.library() {
        println!("Executing first run");
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(HotReloadPlugin)> = lib
                .get("hot_reload_internal_main".as_bytes())
                .unwrap_or_else(|_| panic!("Can't find main function",));
            println!("Run App Thread: {:?}", std::thread::current().id());
            func(HotReloadPlugin::new(library_paths.clone()));
        };
    } else {
        eprint!("Library still somehow missing");
    }
    println!("Got to the end for some reason...");

    let _ = end_watch_tx.send(EndWatch);
}

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
            .init_resource::<HotReload>()
            .init_resource::<ReloadableAppContents>()
            .init_resource::<ReloadableAppCleanupData>()
            .init_resource::<ReplacableResourceStore>()
            .init_resource::<ReplacableComponentStore>()
            .add_event::<HotReloadEvent>()
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
