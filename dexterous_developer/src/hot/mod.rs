mod watch;
use watch::*;

use crate::{
    internal_shared::{lib_path_set::LibPathSet, update_lib::get_initial_library},
    HotReloadOptions, PluginSet,
};

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
        // SAFETY: The function we are calling has to respect rust ownership semantics, and takes ownership of the HotReloadPlugin. We can have high certainty thanks to our control over the compilation of that library - and knowing that it is in fact a rust library.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(LibPathSet, PluginSet)> = lib
                .get("dexterous_developer_internal_main".as_bytes())
                .unwrap_or_else(|_| panic!("Can't find main function",));
            func(library_paths.clone(), options.initial_plugins);
        };
    } else {
        eprint!("Library still somehow missing");
    }
    println!("Got to the end for some reason...");

    let _ = end_watch_tx.send(EndWatch);
}

#[cfg(not(feature = "hot_internal"))]
mod inner {
    pub struct ReloadableAppContents;
    impl crate::private::ReloadableAppSealed for ReloadableAppContents {}

    impl crate::ReloadableApp for ReloadableAppContents {
        fn add_systems<M, L: bevy::ecs::schedule::ScheduleLabel + Eq + std::hash::Hash + Clone>(
            &mut self,
            _schedule: L,
            _systems: impl bevy::prelude::IntoSystemConfigs<M>,
        ) -> &mut Self {
            todo!()
        }

        fn insert_replacable_resource<R: crate::ReplacableResource>(&mut self) -> &mut Self {
            todo!()
        }

        fn reset_resource<R: bevy::prelude::Resource + Default>(&mut self) -> &mut Self {
            todo!()
        }

        fn reset_resource_to_value<R: bevy::prelude::Resource + Clone>(
            &mut self,
            _value: R,
        ) -> &mut Self {
            todo!()
        }

        fn register_replacable_component<C: crate::ReplacableComponent>(&mut self) -> &mut Self {
            todo!()
        }

        fn clear_marked_on_reload<C: bevy::prelude::Component>(&mut self) -> &mut Self {
            todo!()
        }

        fn reset_setup<C: bevy::prelude::Component, M>(
            &mut self,
            _systems: impl bevy::prelude::IntoSystemConfigs<M>,
        ) -> &mut Self {
            todo!()
        }

        fn reset_setup_in_state<C: bevy::prelude::Component, S: bevy::prelude::States, M>(
            &mut self,
            _state: S,
            _systems: impl bevy::prelude::IntoSystemConfigs<M>,
        ) -> &mut Self {
            todo!()
        }
    }
}

#[cfg(not(feature = "hot_internal"))]
pub use inner::ReloadableAppContents;
