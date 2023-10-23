mod hot_reload_internal;
mod reload_systems;
mod reloadable_app;
mod replacable_types;
mod schedules;

use std::marker::PhantomData;

use bevy::app::PluginGroupBuilder;
use bevy::ecs::prelude::*;

use bevy::prelude::{App, First, Plugin, PreStartup, Update};

use bevy::utils::Instant;

use bevy::log::{debug, info, LogPlugin};

pub extern crate dexterous_developer_macros;
pub extern crate libloading;

use crate::bevy_support::hot_internal::hot_reload_internal::draw_internal_hot_reload;
use crate::bevy_support::hot_internal::reload_systems::{
    toggle_reload_mode, toggle_reloadable_elements,
};
use crate::hot_internal::hot_reload_internal::InternalHotReload;
use crate::internal_shared::lib_path_set::LibPathSet;
pub use crate::types::*;
use crate::{InitializablePlugins, InitializeApp, PluginsReady};
use reload_systems::{reload, update_lib_system};
pub use reloadable_app::ReloadableAppElements;
use replacable_types::{ReplacableComponentStore, ReplacableResourceStore};
use schedules::*;

pub struct HotReloadableAppInitializer<'a>(pub(crate) Option<&'a mut App>, pub(crate) &'a mut App);

pub struct HotReloadablePluginsReady<'a, T>(
    PluginGroupBuilder,
    Option<&'a mut App>,
    PluginGroupBuilder,
    &'a mut App,
    Vec<Box<dyn FnOnce(&mut App)>>,
    PhantomData<T>,
);

impl<'a> InitializeApp<'a> for HotReloadableAppInitializer<'a> {
    type PluginsReady<T: InitializablePlugins> = HotReloadablePluginsReady<'a, T>;

    fn initialize<T: InitializablePlugins>(self) -> Self::PluginsReady<T> {
        let fence = self.0;
        let app = self.1;

        HotReloadablePluginsReady(
            T::initialize_fence(),
            fence,
            T::initialize_hot_app(),
            app,
            Vec::new(),
            PhantomData,
        )
    }
}

impl<'a, T: InitializablePlugins> PluginsReady<'a, T> for HotReloadablePluginsReady<'a, T> {
    fn adjust<F: Fn(PluginGroupBuilder) -> PluginGroupBuilder>(mut self, adjust_fn: F) -> Self {
        self.0 = adjust_fn(self.0);
        self.2 = adjust_fn(self.2);
        self
    }

    fn app(self) -> &'a mut App {
        if let Some(fence) = self.1 {
            fence.add_plugins(self.0);

            for mod_fn in self.4.into_iter() {
                mod_fn(fence);
            }
        }

        self.3.add_plugins(self.2).set_runner(|mut app| {
            app.update();
        })
    }

    fn modify_fence<F: 'static + FnOnce(&mut App)>(mut self, fence_fn: F) -> Self {
        if self.1.is_some() {
            self.4.push(Box::new(fence_fn));
        }
        self
    }
}

pub fn build_reloadable_frame(
    libs: std::ffi::CString,
    watch_closure: fn() -> (),
    initialize_app: impl Fn(HotReloadableAppInitializer),
) {
    let plugin = HotReloadPlugin::new(libs, watch_closure);

    let mut fence = App::new();
    let mut inner = App::new();

    let initializer = HotReloadableAppInitializer(Some(&mut fence), &mut inner);

    initialize_app(initializer);

    fence.insert_non_send_resource(HotReloadInnerApp { app: Some(inner) });

    fence.add_plugins(plugin);

    fence.run();
}

struct HotReloadInnerApp {
    pub app: Option<App>,
}
pub struct HotReloadPlugin(LibPathSet, fn() -> ());

impl HotReloadPlugin {
    pub fn new(libs: std::ffi::CString, closure: fn() -> ()) -> Self {
        info!("Building Hot Reload Plugin");
        let libs = libs.to_string_lossy().to_string();
        debug!("Lib at path: {libs}");
        Self(LibPathSet::new(libs), closure)
    }
}

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LogPlugin::default());
        debug!(
            "Build Hot Reload Plugin Thread: {:?}",
            std::thread::current().id()
        );

        debug!("Schedules ready");

        let lib_path = self.0.library_path();

        debug!("Got lib path");

        let hot_reload = InternalHotReload {
            library: None,
            last_lib: None,
            updated_this_frame: true,
            last_update_time: Instant::now(),
            last_update_date_time: chrono::Local::now(),
            libs: LibPathSet::new(lib_path),
        };

        debug!("Set up internal hot reload resources");

        let watcher = {
            let watch = self.1;
            move || {
                debug!("Calling Watch Function");
                watch();
            }
        };

        debug!("Watcher set up triggered");

        debug!("scheduled attached");

        app.init_resource::<ReloadableAppElements>()
            .init_resource::<ReplacableResourceStore>()
            .init_resource::<ReplacableComponentStore>()
            .insert_resource(hot_reload);
        debug!("Added resources to app");

        app.add_systems(PreStartup, (watcher, reload))
            .add_systems(First, (update_lib_system, reload).chain())
            .add_systems(
                Update,
                (
                    draw_internal_hot_reload,
                    toggle_reload_mode,
                    toggle_reloadable_elements,
                    reload_systems::run_update,
                ),
            );
        debug!("Finished build");
    }
}
