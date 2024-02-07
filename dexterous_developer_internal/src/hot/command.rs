use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Error};

use debounce::EventDebouncer;
use notify::{RecursiveMode, Watcher};
use tracing::{debug, error, info, trace};

use crate::{
    hot::{
        build_settings::PackageOrExample,
        singleton::{BUILD_SETTINGS, WATCHER},
    },
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
        example,
        lib_name,
        watch_folders,
        target_folder,
        features,
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

    let mut features = features
        .iter()
        .cloned()
        .chain([
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

    debug!("Getting metadata...");

    let metadata = get_metadata.exec()?;

    if let Some(metadata) = metadata.workspace_metadata.get("hot_reload_features") {
        if let Some(metadata) = metadata.as_array() {
            let mut new_features = metadata.iter().filter_map(|feature| feature.as_str().map(|v| v.to_string())).collect();
            features.append(&mut new_features);
        } else if let Some(feature) = metadata.as_str() {
            features.insert(feature.to_string());
        }
    }

    debug!("Got metadata");

    let packages = metadata.packages.iter();

    let libs = packages.filter_map(|pkg| {
        if let Some(package) = package.as_ref() {
            let pkg = &pkg.name;
            if pkg != package.as_str() {
                return None;
            }
        }
        if let Some(example) = example.as_ref() {
            pkg.targets
                .iter()
                .filter(|p| p.is_example() && p.name == example.as_str())
                .find(|p| {
                    p.crate_types
                        .iter()
                        .map(|v| v.as_str())
                        .any(|v| v == "dylib")
                })
                .map(|p| (PackageOrExample::Example(p.name.clone()), p, pkg))
        } else {
            pkg.targets
                .iter()
                .filter(|p| !(p.is_example() || p.is_bench() || p.is_test()))
                .find(|p| {
                    p.crate_types
                        .iter()
                        .map(|v| v.as_str())
                        .any(|v| v == "dylib")
                })
                .map(|p| (PackageOrExample::Package(pkg.name.clone()), p, pkg))
        }
    });

    let libs: Vec<_> = if let Some(library) = lib_name.as_ref() {
        libs.filter(|v| v.1.name == library.as_str()).collect()
    } else {
        libs.collect()
    };

    if libs.len() > 1 {
        bail!("Workspace contains multiple libraries - please set the one you want with the --package option");
    }

    let Some((pkg_or_example, lib, pkg)) = libs.first() else {
        bail!("Workspace contains no matching libraries");
    };

    if let Some(metadata) = pkg.metadata.get("hot_reload_features") {
        if let Some(metadata) = metadata.as_array() {
            let mut new_features = metadata.iter().filter_map(|feature| feature.as_str().map(|v| v.to_string())).collect();
            features.append(&mut new_features);
        } else if let Some(feature) = metadata.as_str() {
            features.insert(feature.to_string());
        }
    }

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

    debug!("Setting up rustc request");
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
    super::env::set_envs(&mut rustc, build_target.as_ref())?;

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
        watch_folders: if watch_folders.is_empty() {
            let set: HashSet<PathBuf> = [
                lib.src_path
                    .clone()
                    .into_std_path_buf()
                    .parent()
                    .map(|v| v.to_path_buf())
                    .ok_or(Error::msg("Couldn't find source directory to watch"))?,
                pkg.manifest_path
                    .clone()
                    .into_std_path_buf()
                    .parent()
                    .ok_or(Error::msg("Couldn't find source directory to watch"))?
                    .join("src"),
            ]
            .iter()
            .map(|v| v.to_owned())
            .collect();

            set.into_iter().collect()
        } else {
            watch_folders.clone()
        },
        manifest: manifest_path.clone(),
        package: pkg_or_example.clone(),
        features: features.into_iter().collect::<Vec<_>>().join(","),
        target_folder: target_folder.as_ref().cloned().map(|mut v| {
            if v.ends_with("debug") {
                v.pop();
            }
            v
        }),
        out_target,
        build_target: build_target.as_ref().copied(),
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
    let watch_folders = settings.watch_folders.clone();

    for watch_folder in watch_folders.iter() {
        let watch_folder = watch_folder.clone();
        let watching_tx = watching_tx.clone();
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
    }
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

    let cargo = super::env::cargo_command(build_target.as_ref())?;

    let mut command = Command::new(cargo);
    let build_command = super::env::set_envs(&mut command, build_target.as_ref())?;
    command.args(build_command).arg("--profile").arg("dev");

    match package {
        crate::hot::build_settings::PackageOrExample::Package(package) => {
            command.arg("-p").arg(package.as_str());
        }
        crate::hot::build_settings::PackageOrExample::Example(example) => {
            command.arg("--example").arg(example.as_str());
        }
    }

    command
        .arg("--lib")
        .arg("--features")
        .arg(features)
        .arg("--message-format=json-render-diagnostics");

    let mut root_directory = std::env::current_dir()?;

    if let Some(manifest) = manifest {
        command.arg("--manifest-path").arg(manifest.as_os_str());
        root_directory = manifest.to_owned();
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
                info!("Build finished: {finished:?}");
                succeeded = finished.success;
            }
            cargo_metadata::Message::CompilerMessage(message) => {
                info!("Compiler: {}", message.to_string());
            }
            _ => {
                info!("Compilation Message: {message:?}");
            }
        }
    }

    let result = child.wait()?;

    if result.success() && succeeded {
        debug!("Copying built files");
        let mut moved: Vec<PathBuf> = vec![];
        let mut deps_paths = HashSet::new();
        for path in artifacts {
            trace!("Checking {path:?}");
            let path = if !path.exists() {
                trace!("path doesn't exist");
                let path_str = if cargo == "cross" {
                    trace!("Cross build - adjusting path");
                    format!(".{}", path.to_string_lossy())
                } else {
                    path.to_string_lossy().to_string()
                };
                root_directory.join(path_str)
            } else {
                path
            };

            if !path.exists() {
                error!("{path:?} doesn't exist - skipping this file");
                continue;
            }

            debug!("Checking {path:?} for copy");

            let Some(parent) = path.parent() else {
                trace!("{path:?} has no parent");
                continue;
            };
            let Some(filename) = path.file_name() else {
                trace!("{path:?} has no file name");
                continue;
            };
            let Some(stem) = path.file_stem() else {
                trace!("{path:?} has no stem");
                continue;
            };
            let stem = stem.to_string_lossy().to_string();
            let Some(extension) = path.extension() else {
                trace!("{path:?} has no extension");
                continue;
            };
            let extension = extension.to_string_lossy().to_string();
            trace!("file has stem {stem} and extension {extension}");

            let parent_str = parent.to_string_lossy();
            trace!("File parent is: {parent_str}");

            if !parent_str.contains("deps") {
                let deps = if parent_str.ends_with("examples") {
                    parent.to_path_buf()
                } else {
                    let p = parent.join("deps");
                    if !p.exists() {
                        continue;
                    }
                    deps_paths.insert(p.clone());
                    p
                };
                let deps_path = deps.join(filename);
                if deps_path.exists() {
                    trace!("{deps_path:?} exists - using it instead of {path:?}");
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
                    trace!("Searching for deps file for {path:?}");
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
                        trace!("Found {found_file:?} in deps");
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

        for path in deps_paths {
            let files = std::fs::read_dir(&path)
                .context(format!("Attempting to read directory: {path:?}"))?;
            for file in files {
                let Ok(file) = file else {
                    bail!("Couldn't get next file in {path:?}");
                };
                let file_type = file.file_type()?;
                let path = file.path();
                let Some(extension) = path.extension() else {
                    continue;
                };
                if file_type.is_file()
                    && (extension == "dll" || extension == "so" || extension == "dylib")
                {
                    let out_path = out_target.join(file.file_name());
                    if !out_path.exists() {
                        moved.push(out_path.clone());
                        std::fs::copy(path, out_path)?;
                    }
                }
            }
        }

        #[cfg(feature = "cli")]
        {
            if let Some(sender) = settings.updated_file_channel.as_ref() {
                let _ = sender.send(crate::HotReloadMessage::UpdatedLibs(
                    moved
                        .iter()
                        .filter_map(|v| (v.file_name().map(|f| (v, f))))
                        .filter_map(|(path, name)| std::fs::read(path).ok().map(|f| (f, name)))
                        .map(|(f, name)| {
                            let hash = blake3::hash(&f);

                            (name, hash.as_bytes().to_owned())
                        })
                        .map(|(v, h)| (v.to_string_lossy().to_string(), h))
                        .collect(),
                ));
            }
        }
        info!("Build completed");
    } else {
        bail!("Failed to build");
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
