mod watch;
use watch::*;

use crate::{
    internal_shared::{lib_path_set::LibPathSet, update_lib::get_initial_library},
    HotReloadOptions,
};

pub fn run_reloadabe_app(options: HotReloadOptions) {
    let library_paths = LibPathSet::new(&options).unwrap();

    let _ = std::fs::remove_file(library_paths.library_path());

    let build_command = create_build_command(&library_paths, &options.features);

    match first_exec(&build_command) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Initial Build Failed:");
            eprintln!("{err:?}");
            std::process::exit(1);
        }
    };

    setup_environment_variables(&library_paths);

    let lib = get_initial_library(&library_paths);

    if let Some(lib) = lib.library() {
        println!("Executing first run");
        // SAFETY: The function we are calling has to respect rust ownership semantics, and takes ownership of the HotReloadPlugin. We can have high certainty thanks to our control over the compilation of that library - and knowing that it is in fact a rust library.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(LibPathSet, String)> = lib
                .get("dexterous_developer_internal_main".as_bytes())
                .unwrap_or_else(|_| panic!("Can't find main function",));
            func(library_paths.clone(), build_command);
        };
    } else {
        eprint!("Library still somehow missing");
    }
    println!("Got to the end for some reason...");
}

fn setup_environment_variables(library_paths: &LibPathSet) {
    let target_path = library_paths.folder.as_os_str().to_str().unwrap();
    let mut target_deps_path = library_paths.folder.clone();
    target_deps_path.push("deps");
    let target_deps_path = target_deps_path.as_os_str().to_str().unwrap();

    if let Ok(path) = std::env::var("PATH") {
        std::env::set_var("PATH", format!("{path};{target_path};{target_deps_path}"));
        println!("Set PATH to {:?}", std::env::var("PATH"));
    }
    if let Ok(path) = std::env::var("DYLD_FALLBACK_LIBRARY_PATH") {
        std::env::set_var(
            "DYLD_FALLBACK_LIBRARY_PATH",
            format!("{path}:{target_path}:{target_deps_path}"),
        );
        println!(
            "Set DYLD_FALLBACK_LIBRARY_PATH to {:?}",
            std::env::var("DYLD_FALLBACK_LIBRARY_PATH")
        );
    }
    if let Ok(path) = std::env::var("LD_LIBRARY_PATH") {
        std::env::set_var(
            "LD_LIBRARY_PATH",
            format!("{path}:{target_path}:{target_deps_path}"),
        );
        println!(
            "Set LD_LIBRARY_PATH to {:?}",
            std::env::var("LD_LIBRARY_PATH")
        );
    }
}

#[cfg(all(not(feature = "hot_internal"), feature = "bevy"))]
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

#[cfg(all(not(feature = "hot_internal"), feature = "bevy"))]
pub use inner::ReloadableAppContents;
