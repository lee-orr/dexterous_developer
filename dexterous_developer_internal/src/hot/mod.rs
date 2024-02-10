mod build_settings;
mod command;
mod env;
mod singleton;
use std::{fmt::Display, process::Command, sync::Once};

use tracing::{debug, error, info};

use command::*;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    hot::singleton::{load_build_settings, BUILD_SETTINGS},
    internal_shared::{update_lib::get_initial_library, LibPathSet, LibraryHolder},
    HotReloadOptions,
};

pub use self::build_settings::HotReloadMessage;

#[derive(Clone, Debug)]
pub struct Error {
    msg: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Error Running App - {}", &self.msg))
    }
}

static RUNNER: Once = Once::new();

pub fn run_reloadabe_app(options: HotReloadOptions) -> Result<(), Error> {
    let mut outcome = None;
    RUNNER.call_once(|| {
        tracing_subscriber::registry()
            .with(fmt::layer())
            .with(EnvFilter::from_default_env())
            .init();
        if let Ok(settings) = std::env::var("DEXTEROUS_BUILD_SETTINGS") {
            info!("Running based on DEXTEROUS_BUILD_SETTINGS env");
            outcome = Some(run_reloadable_from_env(settings));
        } else {
            info!("Running based on options");
            outcome = Some(run_reloadabe_app_inner(options));
        }
    });

    match outcome {
        Some(outcome) => outcome,
        None => Err(Error {
            msg: "Called run_reloadable_app too many times".to_string(),
        }),
    }
}

fn run_reloadabe_app_inner(options: HotReloadOptions) -> Result<(), Error> {
    let (settings, paths) =
        setup_build_settings(&options).expect("Couldn't get initial build settings");

    match setup_build_setting_environment(settings, paths)
        .expect("Couldn't set up build settings in environment")
    {
        BuildSettingsReady::LibraryPath(library_paths) => {
            run_app_with_path(library_paths).map_err(|e| Error { msg: e.to_string() })
        }
        BuildSettingsReady::RequiredEnvChange(var, val) => {
            info!("Requires env change");
            let current = std::env::current_exe().expect("Can't get current executable");
            debug!("Setting {var} to {val}");
            let result = Command::new(current).env(var, val).status();
            let result = result.map_err(|_| Error {
                msg: "Couldn't execute app".to_string(),
            })?;
            if !result.success() {
                return Err(Error {
                    msg: "Error while running app".to_string(),
                });
            }
            Ok(())
        }
    }
}

fn run_app_with_path(library_paths: crate::internal_shared::LibPathSet) -> Result<(), Error> {
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

    run_from_file(library_paths, lib, run_watcher).map_err(|e| Error { msg: e.to_string() })
}

fn run_reloadable_from_env(settings: String) -> Result<(), Error> {
    println!("Running from env");
    let dir = std::env::current_dir();
    println!("Current directory: {:?}", dir);
    debug!("__Envvironment Variables__");
    for (key, val) in std::env::vars_os() {
        debug!("{key:?}={val:?}");
    }
    debug!("Got Environment\n");

    let library_paths =
        load_build_settings(settings).expect("Couldn't load build settings from env");
    run_app_with_path(library_paths)
}

#[cfg(feature = "cli")]
pub fn watch_reloadable(
    options: HotReloadOptions,
    update_channel: tokio::sync::broadcast::Sender<HotReloadMessage>,
) -> anyhow::Result<(std::path::PathBuf, std::path::PathBuf)> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let (mut settings, paths) = setup_build_settings(&options)?;
    let lib_path = settings.lib_path.clone();
    let lib_dir = settings.out_target.clone();

    if !lib_dir.exists() {
        let _ = std::fs::create_dir_all(lib_dir.as_path());
    }

    for dir in paths.iter() {
        if dir.as_path() != lib_dir.as_path() && dir.exists() {
            tracing::trace!("Checking lib path {dir:?}");
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
                    tracing::trace!("Moving {path:?} to {new_file:?}");
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
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let library_paths = LibPathSet::new(library_path.as_path());

    let lib = get_initial_library(&library_paths).map_err(anyhow::Error::msg)?;

    run_from_file(library_paths, lib, null_watcher)?;
    Ok(())
}

#[cfg(feature = "cli")]
fn null_watcher() {}

#[cfg(feature = "cli")]
pub async fn run_existing_library(library_path: std::path::PathBuf) -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let library_paths = LibPathSet::new(library_path.as_path());

    let lib = get_initial_library(&library_paths).map_err(anyhow::Error::msg)?;

    run_from_file(library_paths, lib, null_watcher)?;
    Ok(())
}

#[cfg(feature = "cli")]
pub fn compile_reloadable_libraries(
    options: HotReloadOptions,
    lib_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context;

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let (mut settings, paths) = setup_build_settings(&options)?;
    let lib_path = settings.lib_path.clone();

    let lib_dir = if lib_dir.is_absolute() {
        lib_dir.to_path_buf()
    } else {
        std::env::current_dir()?.join(lib_dir)
    };
    settings.out_target = lib_dir.clone();

    if !lib_dir.exists() {
        let _ = std::fs::create_dir_all(&lib_dir);
    }

    let lib_extension = lib_path
        .extension()
        .context("No extension for the library")?
        .to_string_lossy();

    for dir in paths.iter() {
        if dir.as_path() != lib_dir && dir.exists() {
            tracing::trace!("Checking lib path {dir:?}");
            for file in (dir.read_dir()?).flatten() {
                let path = file.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if path.is_file() && (extension == lib_extension) {
                    let new_file = lib_dir.join(file.file_name());
                    tracing::trace!("Moving {path:?} to {new_file:?}");
                    std::fs::copy(path, new_file)?;
                }
            }
        }
    }

    first_exec(&settings).context("Build failed")?;
    let lib_path = lib_dir.join(lib_path.file_name().context("Lib must have a file name")?);
    if !lib_path.exists() {
        anyhow::bail!("{lib_path:?} wasn't built");
    }

    std::fs::copy(
        &lib_path,
        lib_path.with_extension(format!("{lib_extension}.backup")),
    )?;

    Ok(lib_dir.to_path_buf())
}
