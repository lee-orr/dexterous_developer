mod command;
use std::sync::Once;

use command::*;

use crate::{
    internal_shared::{lib_path_set::LibPathSet, update_lib::get_initial_library},
    HotReloadOptions,
};

static RUNNER: Once = Once::new();

pub fn run_reloadabe_app(options: HotReloadOptions) {
    RUNNER.call_once(|| {
        run_reloadabe_app_inner(options);
    });
}

fn run_reloadabe_app_inner(options: HotReloadOptions) {
    let library_paths = LibPathSet::new(&options).unwrap();

    let _ = std::fs::remove_file(library_paths.library_path());

    match first_exec(
        &options.package,
        &options.lib_name,
        &options.watch_folder,
        &options.features,
    ) {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Initial Build Failed:");
            eprintln!("{err:?}");
            eprintln!("{:?}", err.source());
            std::process::exit(1);
        }
    };

    let lib = get_initial_library(&library_paths);

    if let Some(lib) = lib.library() {
        println!("Executing first run");
        // SAFETY: The function we are calling has to respect rust ownership semantics, and takes ownership of the HotReloadPlugin. We can have high certainty thanks to our control over the compilation of that library - and knowing that it is in fact a rust library.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(LibPathSet, fn() -> ())> = lib
                .get("dexterous_developer_internal_main".as_bytes())
                .unwrap_or_else(|_| panic!("Can't find main function",));

            func(library_paths.clone(), run_watcher);
        };
    } else {
        eprint!("Library still somehow missing");
    }
    println!("Got to the end for some reason...");
}
