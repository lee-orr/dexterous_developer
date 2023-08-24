mod command;
use std::{path::Path, sync::Once};

use command::*;

use crate::{internal_shared::update_lib::get_initial_library, HotReloadOptions};

static RUNNER: Once = Once::new();

pub fn run_reloadabe_app(options: HotReloadOptions) {
    RUNNER.call_once(|| {
        run_reloadabe_app_inner(options);
    });
}

fn run_reloadabe_app_inner(options: HotReloadOptions) {
    let library_paths =
        setup_build_settings(&options).expect("Couldn't get initial build settings");

    let _ = std::fs::remove_file(library_paths.library_path());

    match first_exec() {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Initial Build Failed:");
            eprintln!("{err:?}");
            eprintln!("{:?}", err.source());
            std::process::exit(1);
        }
    };

    let lib = get_initial_library(&library_paths).expect("Failed to find library");

    if let Some(lib) = lib.library() {
        println!("Executing first run");
        // SAFETY: The function we are calling has to respect rust ownership semantics, and takes ownership of the HotReloadPlugin. We can have high certainty thanks to our control over the compilation of that library - and knowing that it is in fact a rust library.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(&Path, fn() -> ())> = lib
                .get("dexterous_developer_internal_main".as_bytes())
                .unwrap_or_else(|_| panic!("Can't find main function",));

            func(library_paths.library_path().as_path(), run_watcher);
        };
    } else {
        eprint!("Library still somehow missing");
    }
    println!("Got to the end for some reason...");
}
