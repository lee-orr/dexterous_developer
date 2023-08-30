use std::{
    collections::BTreeSet,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::{bail, Error};

use debounce::EventDebouncer;
use log::{debug, error, info};
use notify::{RecursiveMode, Watcher};

use crate::{
    hot::singleton::{BUILD_SETTINGS, WATCHER},
    internal_shared::cargo_path_utils,
    internal_shared::LibPathSet,
    HotReloadOptions,
};

use super::build_settings::BuildSettings;

pub enum BuildSettingsReady {
    LibraryPath(LibPathSet),
    RequiredEnvChange(String, String),
}

pub(crate) fn setup_build_settings(
    options: &HotReloadOptions,
) -> anyhow::Result<(BuildSettings, BTreeSet<PathBuf>)> {
    let HotReloadOptions {
        manifest_path,
        package,
        lib_name,
        watch_folder,
        target_folder,
        features,
        prefer_mold,
        build_target,
        ..
    } = options;

    if let Some(l) = manifest_path.as_ref() {
        info!("Using manifest  {}", l.to_string_lossy());
    }

    if let Some(p) = package.as_ref() {
        info!("Using Package {p}");
    }

    if let Some(l) = lib_name.as_ref() {
        info!("Using library {l}");
    }

    if let Some(l) = target_folder.as_ref() {
        info!("Target at target folder {}", l.to_string_lossy());
    }

    if let Some(l) = target_folder.as_ref() {
        info!("Watching source at  {}", l.to_string_lossy());
    }

    if let Some(l) = build_target.as_ref() {
        info!("For platform {l}");
    }

    info!("Compiling with features: {}", features.join(", "));

    super::env::set_envs(*prefer_mold)?;

    let features = features
        .iter()
        .cloned()
        .chain([
            "bevy/dynamic_linking".to_string(),
            "dexterous_developer/hot_internal".to_string(),
        ])
        .collect::<BTreeSet<_>>();

    let mut get_metadata = cargo_metadata::MetadataCommand::new();
    get_metadata.no_deps();
    if let Some(manifest) = manifest_path {
        get_metadata.manifest_path(manifest);
    }
    get_metadata.features(cargo_metadata::CargoOpt::SomeFeatures(
        features.iter().cloned().collect(),
    ));

    if let Some(target) = target_folder {
        get_metadata.env("CARGO_BUILD_TARGET_DIR", target.as_os_str());
    }

    let metadata = get_metadata.exec()?;

    let packages = metadata.packages.iter();

    let libs = packages.filter_map(|pkg| {
        if let Some(package) = package.as_ref() {
            let pkg = &pkg.name;
            debug!("Checking package name: {package} - {pkg}");
            if pkg != package.as_str() {
                return None;
            }
        }
        pkg.targets
            .iter()
            .find(|p| {
                let result = p.crate_types.contains(&String::from("dylib"));
                debug!(
                    "Checking {} @ {} - {:?} {result}",
                    p.name, pkg.name, p.crate_types
                );
                result
            })
            .map(|p| (pkg, p))
    });

    let libs: Vec<_> = if let Some(library) = lib_name.as_ref() {
        libs.filter(|v| v.1.name == library.as_str()).collect()
    } else {
        libs.collect()
    };

    if libs.len() > 1 {
        bail!("Workspace contains multiple libraries - please set the one you want with the --package option");
    }

    let Some((pkg, lib)) = libs.first() else {
        bail!("Workspace contains no matching libraries");
    };

    let mut target_path = if let Some(target) = target_folder {
        target.clone()
    } else {
        metadata.target_directory.into_std_path_buf()
    };

    if target_path.ends_with("debug") {
        target_path.pop();
    }

    let out_target = target_path.join("hot");
    target_path.push("debug");

    let mut rustc = Command::new("rustc");
    rustc
        .env_remove("LD_DEBUG")
        .arg(lib.src_path.as_os_str())
        .arg("--crate-type")
        .arg("dylib")
        .arg("--crate-name")
        .arg(&lib.name)
        .arg("--print=sysroot")
        .arg("--print=target-libdir")
        .arg("--print=native-static-libs")
        .arg("--print=file-names");

    if let Some(build_target) = build_target {
        rustc.arg("--target").arg(build_target.as_str());
    }

    let cmd_string = print_command(&rustc);

    debug!("Run rustc {rustc:#?} - {cmd_string}");

    let rustc_output = rustc.output()?;
    let output = std::str::from_utf8(&rustc_output.stdout)?;
    let errout = std::str::from_utf8(&rustc_output.stderr)?;

    if !rustc_output.status.success() {
        bail!("rustc status {:#?}\n{errout}", rustc_output.status);
    }

    debug!("rustc output {output}");
    debug!("rustc err {errout}");

    let paths = output
        .lines()
        .chain(errout.lines())
        .filter(|v| !v.contains("Compiling ") && !v.contains("Finished "))
        .map(PathBuf::from)
        .chain([out_target.clone()])
        .collect::<BTreeSet<_>>();

    debug!("Paths found {paths:?}");

    let lib_file_name = paths
        .iter()
        .find(|p| {
            p.extension()
                .filter(|p| p.to_string_lossy() != "rlib")
                .is_some()
        })
        .cloned()
        .ok_or(anyhow::Error::msg("No file name for lib"))?;

    let lib_path = out_target.join(lib_file_name);

    debug!("Filtered paths {paths:?}");

    let settings = BuildSettings {
        lib_path: lib_path.clone(),
        watch_folder: watch_folder
            .as_ref()
            .cloned()
            .or_else(|| {
                lib.src_path
                    .clone()
                    .into_std_path_buf()
                    .parent()
                    .map(|v| v.to_path_buf())
            })
            .ok_or(Error::msg("Couldn't find source directory to watch"))?,
        manifest: manifest_path.clone(),
        package: pkg.name.clone().clone(),
        features: features.into_iter().collect::<Vec<_>>().join(","),
        target_folder: target_folder.as_ref().cloned().map(|mut v| {
            if v.ends_with("debug") {
                v.pop();
            }
            v
        }),
        out_target,
        build_target: build_target.as_ref().cloned(),
        ..Default::default()
    };

    Ok((settings, paths))
}

pub(crate) fn setup_build_setting_environment(
    settings: BuildSettings,
    paths: BTreeSet<PathBuf>,
) -> anyhow::Result<BuildSettingsReady> {
    // SET ENVIRONMENT VARS HERE!
    let dylib_path_var = cargo_path_utils::dylib_path_envvar();
    let mut env_paths = cargo_path_utils::dylib_path();
    let paths = paths
        .into_iter()
        .filter(|v| v.extension().is_none() && v.is_absolute())
        .collect::<Vec<_>>();

    if paths.iter().any(|v| !env_paths.contains(v)) {
        for path in paths.iter() {
            if !path.exists() {
                std::fs::create_dir_all(path)?;
            }
        }

        {
            let mut collect = paths.clone();
            env_paths.append(&mut collect);
        }

        let env_paths = env_paths
            .into_iter()
            .filter(|v| !v.as_os_str().is_empty())
            .collect::<BTreeSet<_>>();

        let os_paths = std::env::join_paths(env_paths)?;

        std::env::set_var(dylib_path_var, os_paths.as_os_str());

        debug!(
            "Environment Variables Set {:?}",
            std::env::var(dylib_path_var)
        );

        let settings = settings.to_string();

        debug!("Setting DEXTEROUS_BUILD_SETTINGS env to {settings}");
        std::env::set_var("DEXTEROUS_BUILD_SETTINGS", &settings);

        return Ok(BuildSettingsReady::RequiredEnvChange(
            dylib_path_var.to_string(),
            os_paths.to_string_lossy().to_string(),
        ));
    }

    BUILD_SETTINGS
        .set(settings.clone())
        .map_err(|_| Error::msg("Build settings already set"))?;

    info!("Finished Setting Up");

    Ok(BuildSettingsReady::LibraryPath(LibPathSet::new(
        settings.lib_path.clone(),
    )))
}

pub(crate) fn first_exec(settings: &BuildSettings) -> anyhow::Result<()> {
    info!("First Execution");
    rebuild_internal(settings)
}

pub(crate) fn run_watcher() {
    debug!("run watcher called");
    WATCHER.call_once(|| {
        debug!("Getting Settings");
        let Some(settings) = BUILD_SETTINGS.get() else {
            panic!("Couldn't get settings...");
        };
        debug!("Setting up watcher");
        if let Err(e) = run_watcher_with_settings(settings) {
            error!("Couldn't set up watcher - {e:?}");
        };
    });
}

pub(crate) fn run_watcher_with_settings(settings: &BuildSettings) -> anyhow::Result<()> {
    info!("Getting watch settings");
    let delay = Duration::from_secs(2);
    let (watching_tx, watching_rx) = mpsc::channel::<()>();
    let watch_folder = settings.watch_folder.clone();
    let settings = settings.clone();

    info!("Setting up watcher with {watch_folder:?}");
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        debug!("Spawned watch thread");
        let debounced = EventDebouncer::new(delay, move |_data: ()| {
            debug!("Files Changed");
            let _ = tx.send(());
        });
        debug!("Debouncer set up with delay {delay:?}");

        let Ok(mut watcher) = notify::recommended_watcher(move |_| {
            debug!("Got Watch Event");
            debounced.put(());
        }) else {
            error!("Couldn't setup watcher");
            return;
        };
        debug!("Watcher response set up");

        if let Err(e) = watcher.watch(watch_folder.as_path(), RecursiveMode::Recursive) {
            error!("Error watching files: {e:?}");
            return;
        }

        watching_tx.send(()).expect("Couldn't send watch");

        while rx.recv().is_ok() {
            rebuild(&settings);
        }
    });
    info!("Starting watch receiver");
    watching_rx.recv()?;
    info!("Watching...");
    Ok(())
}

fn rebuild(settings: &BuildSettings) {
    if let Err(e) = rebuild_internal(settings) {
        error!("Failed to rebuild: {e:?}");
    }
}

fn rebuild_internal(settings: &BuildSettings) -> anyhow::Result<()> {
    let BuildSettings {
        manifest,
        features,
        package,
        out_target,
        lib_path,
        build_target,
        ..
    } = settings;

    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--profile")
        .arg("dev")
        .arg("-p")
        .arg(package.as_str())
        .arg("--lib")
        .arg("--features")
        .arg(features)
        .arg("--message-format=json");

    if let Some(manifest) = manifest {
        command.arg("--manifest-path").arg(manifest.as_os_str());
    }

    if let Some(build_target) = build_target {
        command.arg("--target").arg(build_target.as_str());
    }

    info!("Command: {}", print_command(&command));

    let mut child = command
        .env_remove("LD_DEBUG")
        .stdout(Stdio::piped())
        .spawn()?;

    let reader = std::io::BufReader::new(
        child
            .stdout
            .take()
            .ok_or(anyhow::Error::msg("Couldn't get stdout"))?,
    );

    let mut succeeded = false;

    let mut artifacts = Vec::with_capacity(20);

    for msg in cargo_metadata::Message::parse_stream(reader) {
        let message = msg?;
        match &message {
            cargo_metadata::Message::CompilerArtifact(artifact) => {
                if artifact.target.crate_types.iter().any(|v| v == "dylib") {
                    for path in artifact.filenames.iter() {
                        artifacts.push(path.clone().into_std_path_buf());
                    }
                }
            }
            cargo_metadata::Message::BuildFinished(finished) => {
                debug!("Build finished: {finished:?}");
                succeeded = finished.success;
            }
            _ => {
                debug!("Compilation Message: {message:?}");
            }
        }
    }

    let result = child.wait()?;

    if result.success() && succeeded {
        let mut moved = vec![];
        for path in artifacts {
            let Some(parent) = path.parent() else {
                continue;
            };
            let Some(filename) = path.file_name() else {
                continue;
            };
            let Some(stem) = path.file_stem() else {
                continue;
            };
            let stem = stem.to_string_lossy().to_string();
            let Some(extension) = path.extension() else {
                continue;
            };
            let extension = extension.to_string_lossy().to_string();
            if parent.to_string_lossy() != "deps" {
                let deps = parent.join("deps");
                let deps_path = deps.join(filename);
                if deps_path.exists() {
                    let out_path = out_target.join(filename);
                    if !out_path.exists() {
                        debug!("Copying from {deps_path:?} to {out_path:?}");
                        moved.push(out_path.clone());
                        std::fs::copy(deps_path, out_path)?;
                    } else {
                        if out_path.as_path() != lib_path.as_path() {
                            debug!("Should only override the hot reloaded library - not any dynamic dependencies");
                            continue;
                        }
                        match std::fs::copy(deps_path, out_path.as_path()) {
                            Ok(_) => {
                                moved.push(out_path.clone());
                                debug!("{out_path:?} replaced");
                            }
                            Err(_e) => error!("Couldn't replace {out_path:?} - using original"),
                        }
                    }
                } else {
                    let mut found_file = None;
                    let Ok(read_dir) = deps.read_dir() else {
                        error!("Couldn't read directory {deps:?}");
                        continue;
                    };
                    for file in read_dir {
                        let file = file?;
                        let filename = file.file_name().to_string_lossy().to_string();
                        if filename.starts_with(&stem) && filename.ends_with(&extension) {
                            if let Some((_, old_time)) = &found_file {
                                let time = file.metadata()?.modified()?;
                                if time > *old_time {
                                    found_file = Some((file.path(), time));
                                }
                            } else {
                                found_file = Some((file.path(), file.metadata()?.modified()?));
                            }
                        }
                    }
                    if let Some((found_file, _)) = found_file {
                        let Some(filename) = found_file.file_name() else {
                            continue;
                        };
                        let out_path = out_target.join(filename);
                        if !out_path.exists() {
                            debug!("Copying from {deps_path:?} to {out_path:?}");
                            moved.push(out_path.clone());
                            std::fs::copy(found_file, out_path)?;
                        } else {
                            if filename.to_string_lossy().starts_with(&format!("{stem}-")) {
                                debug!("Hashed filename - not replacing");
                                continue;
                            }
                            match std::fs::copy(found_file, out_path.as_path()) {
                                Ok(_) => {
                                    debug!("{out_path:?} replaced");
                                    moved.push(out_path.clone());
                                }
                                Err(_e) => {
                                    error!("Couldn't replace {out_path:?} - using original")
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(feature = "cli")]
        {
            if let Some(sender) = settings.updated_file_channel.as_ref() {
                let _ = sender.send(crate::HotReloadMessage::UpdatedPaths(moved));
                let _ = sender.send(crate::HotReloadMessage::RootLibPath(
                    settings.lib_path.clone(),
                ));
            }
        }
        info!("Build completed");
    } else {
        bail!(
            "Failed to build
        env:
        {}",
            std::env::vars()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
}

fn print_command(command: &Command) -> String {
    let args = command
        .get_args()
        .map(|v| v.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let program = command.get_program().to_string_lossy();
    format!("\'{program} {args}\'")
}
