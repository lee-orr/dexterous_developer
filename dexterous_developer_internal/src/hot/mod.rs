mod build_settings;
mod command;
mod env;
mod singleton;
use std::{process::Command, sync::Once};

use log::{debug, error, info};

use command::*;

use crate::{
    hot::singleton::{load_build_settings, BUILD_SETTINGS},
    internal_shared::update_lib::get_initial_library,
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
        setup_build_settings(&options).expect("Couldn't get initial build settings");

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

            func(path, run_watcher);
        };
    } else {
        error!("Library still somehow missing");
    }
    info!("Exiting");
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
pub async fn watch_reloadable(
    options: HotReloadOptions,
    update_channel: tokio::sync::broadcast::Sender<HotReloadMessage>,
) -> anyhow::Result<()> {
    let (mut settings, _) = setup_build_settings(&options)?;
    settings.updated_file_channel = Some(update_channel);
    first_exec(&settings)?;
    run_watcher_with_settings(&settings)?;
    Ok(())
}
