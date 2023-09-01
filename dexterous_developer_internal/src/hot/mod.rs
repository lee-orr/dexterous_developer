mod build_settings;
mod command;
mod env;
mod singleton;
use std::{process::Command, sync::Once};

use log::{debug, error, info};

use command::*;

use crate::{
    hot::singleton::{load_build_settings, BUILD_SETTINGS},
    internal_shared::{update_lib::get_initial_library, LibPathSet, LibraryHolder},
    HotReloadOptions,
};

pub use self::build_settings::HotReloadMessage;

static RUNNER: Once = Once::new();

pub fn run_reloadabe_app(options: HotReloadOptions) {
    RUNNER.call_once(|| {
        println!("Called Run Init");
        let _ = env_logger::try_init();
        if let Ok(settings) = std::env::var("DEXTEROUS_BUILD_SETTINGS") {
            info!("Running based on DEXTEROUS_BUILD_SETTINGS env");
            run_reloadable_from_env(settings);
        } else {
            info!("Running based on options");
            run_reloadabe_app_inner(options);
        }
    });
}

fn run_reloadabe_app_inner(options: HotReloadOptions) {
    let (settings, paths) =
        setup_build_settings(&options, None).expect("Couldn't get initial build settings");

    match setup_build_setting_environment(settings, paths)
        .expect("Couldn't set up build settings in environment")
    {
        BuildSettingsReady::LibraryPath(library_paths) => {
            run_app_with_path(library_paths);
        }
        BuildSettingsReady::RequiredEnvChange(var, val) => {
            info!("Requires env change");
            let current = std::env::current_exe().expect("Can't get current executable");
            debug!("Setting {var} to {val}");
            let result = Command::new(current)
                .env(var, val)
                .status()
                .expect("Couldn't execute executable");
            std::process::exit(result.code().unwrap_or_default());
        }
    }
}

fn run_app_with_path(library_paths: crate::internal_shared::LibPathSet) {
    let _ = std::fs::remove_file(library_paths.library_path());
    let settings = BUILD_SETTINGS
        .get()
        .expect("Couldn't get existing build settings");

    match first_exec(settings) {
        Ok(_) => {}
        Err(err) => {
            error!("Initial Build Failed:");
            error!("{err:?}");
            error!("{:?}", err.source());
            std::process::exit(1);
        }
    };

    let lib = get_initial_library(&library_paths).expect("Failed to find library");

    run_from_file(library_paths, lib, run_watcher).expect("Couldn't run file");
}

fn run_reloadable_from_env(settings: String) {
    debug!("__Envvironment Variables__");
    for (key, val) in std::env::vars_os() {
        debug!("{key:?}={val:?}");
    }
    debug!("Got Environment\n");

    let library_paths =
        load_build_settings(settings).expect("Couldn't load build settings from env");
    run_app_with_path(library_paths);
}

#[cfg(feature = "cli")]
pub fn watch_reloadable(
    options: HotReloadOptions,
    update_channel: tokio::sync::broadcast::Sender<HotReloadMessage>,
    cross_path: Option<std::path::PathBuf>,
) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
    let _ = env_logger::try_init();
    let (mut settings, paths) = setup_build_settings(&options, cross_path.as_ref())?;
    let lib_path = settings.lib_path.clone();
    let lib_dir = settings.out_target.clone();

    if !lib_dir.exists() {
        let _ = std::fs::create_dir_all(lib_dir.as_path());
    }

    for dir in paths.iter() {
        if dir.as_path() != lib_dir.as_path() && dir.exists() {
            log::trace!("Checking lib path {dir:?}");
            for file in (dir.read_dir()?).flatten() {
                let path = file.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if path.is_file()
                    && (extension == "dll" || extension == "dylib" || extension == "so")
                {
                    let new_file = lib_dir.join(file.file_name());
                    log::trace!("Moving {path:?} to {new_file:?}");
                    std::fs::copy(path, new_file)?;
                }
            }
        }
    }

    settings.updated_file_channel = Some(update_channel);
    tokio::spawn(async move {
        first_exec(&settings).expect("Build failed");
        run_watcher_with_settings(&settings).expect("Couldn't run watcher");
    });
    Ok((lib_path, lib_dir))
}

fn run_from_file(
    library_paths: LibPathSet,
    lib: LibraryHolder,
    watcher: fn() -> (),
) -> anyhow::Result<()> {
    if let Some(lib) = lib.library() {
        debug!("Executing first run");
        // SAFETY: The function we are calling has to respect rust ownership semantics, and takes ownership of the HotReloadPlugin. We can have high certainty thanks to our control over the compilation of that library - and knowing that it is in fact a rust library.
        unsafe {
            let func: libloading::Symbol<unsafe extern "system" fn(std::ffi::CString, fn() -> ())> =
                lib.get("dexterous_developer_internal_main".as_bytes())
                    .unwrap_or_else(|_| panic!("Can't find main function",));

            let path =
                std::ffi::CString::new(library_paths.library_path().to_string_lossy().to_string())
                    .expect("Couldn't convert lib path into a C String");

            debug!("Got path {path:?}");

            func(path, watcher);
        };
    } else {
        error!("Library still somehow missing");
    }
    info!("Exiting");
    Ok(())
}

#[cfg(feature = "cli")]
pub async fn run_served_file(library_path: std::path::PathBuf) -> anyhow::Result<()> {
    let _ = env_logger::try_init();

    let library_paths = LibPathSet::new(library_path.as_path());

    let lib = get_initial_library(&library_paths).map_err(anyhow::Error::msg)?;

    run_from_file(library_paths, lib, null_watcher)?;
    Ok(())
}

#[cfg(feature = "cli")]
fn null_watcher() {}
