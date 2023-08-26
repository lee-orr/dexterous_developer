use std::{
    collections::BTreeSet,
    path::PathBuf,
    process::Command,
    sync::{mpsc, Once, OnceLock},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Error};

use debounce::EventDebouncer;
use notify::{RecursiveMode, Watcher};

use crate::{internal_shared::cargo_path_utils, internal_shared::LibPathSet, HotReloadOptions};

struct BuildSettings {
    watch_folder: PathBuf,
    manifest: PathBuf,
    lib_path: PathBuf,
    package: String,
    features: String,
    target_folder: Option<PathBuf>,
    out_folders: Vec<PathBuf>,
}

impl ToString for BuildSettings {
    fn to_string(&self) -> String {
        let BuildSettings {
            watch_folder,
            manifest,
            package,
            features,
            target_folder,
            lib_path,
            out_folders,
        } = self;

        let target = target_folder
            .as_ref()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_default();

        let out_folders = out_folders
            .iter()
            .map(|v| v.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(";;");

        let watch_folder = watch_folder.to_string_lossy();
        let manifest = manifest.to_string_lossy();
        let lib_path = lib_path.to_string_lossy();

        format!("{lib_path}:!:{watch_folder}:!:{manifest}:!:{package}:!:{features}:!:{out_folders}:!:{target}")
    }
}

impl TryFrom<&str> for BuildSettings {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut split = value.split(":!:");
        let lib_path = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no library path"))?;
        let watch_folder = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no watch folder"))?;
        let manifest = split
            .next()
            .map(PathBuf::from)
            .ok_or(Error::msg("no manifest"))?;
        let package = split
            .next()
            .map(|v| v.to_string())
            .ok_or(Error::msg("no package"))?;
        let features = split
            .next()
            .map(|v| v.to_string())
            .ok_or(Error::msg("no features"))?;
        let out_folders = split
            .next()
            .map(|v| v.to_string())
            .ok_or(Error::msg("no out folders"))?;
        let out_folders = out_folders.split(";;").map(PathBuf::from).collect();
        let target_folder = split.next().filter(|v| !v.is_empty()).map(PathBuf::from);

        Ok(BuildSettings {
            lib_path,
            watch_folder,
            manifest,
            package,
            features,
            target_folder,
            out_folders,
        })
    }
}

static BUILD_SETTINGS: OnceLock<BuildSettings> = OnceLock::new();

#[cfg(target_os = "windows")]
const RUSTC_ARGS: [(&str, &str); 3] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTC_LINKER", "rust-lld.exe"),
    ("RUSTFLAGS", "-Zshare-generics=n"),
];
#[cfg(target_os = "linux")]
const RUSTC_ARGS: [(&str, &str); 3] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTC_LINKER", "clang"),
    ("RUSTFLAGS", "-Zshare-generics=y  -Clink-arg=-fuse-ld=lld"),
];
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    (
        "RUSTFLAGS",
        "-Zshare-generics=y -Clink-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.lld",
    ),
];
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    (
        "RUSTFLAGS",
        "-Zshare-generics=y -Clink-arg=-fuse-ld=/usr/local/opt/llvm/bin/ld64.lld",
    ),
];
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    ("RUSTFLAGS", "-Zshare-generics=y"),
];

fn set_envs() -> anyhow::Result<()> {
    for (var, val) in RUSTC_ARGS.iter() {
        if (var == &"RUSTC_LINKER") && which::which(val).is_err() {
            bail!("Linker {val} is not installed");
        } else if val.contains("-fuse-ld=") {
            let mut split = val.split("-fuse-ld=");
            let _ = split.next();
            let after = split.next().ok_or(Error::msg("No value for -fuse-ld="))?;
            which::which(after).context("Can't find lld")?;
        }
        std::env::set_var(var, val);
    }
    Ok(())
}

pub enum BuildSettingsReady {
    LibraryPath(LibPathSet),
    RequiredEnvChange(String, String),
}

pub(crate) fn setup_build_settings(
    options: &HotReloadOptions,
) -> anyhow::Result<BuildSettingsReady> {
    let HotReloadOptions {
        manifest_path,
        package,
        lib_name,
        watch_folder,
        target_folder,
        features,
        set_env: _,
    } = options;

    if let Some(l) = manifest_path.as_ref() {
        println!("Using manifest  {}", l.to_string_lossy());
    }

    if let Some(p) = package.as_ref() {
        println!("Using Package {p}");
    }

    if let Some(l) = lib_name.as_ref() {
        println!("Using library {l}");
    }

    if let Some(l) = target_folder.as_ref() {
        println!("Target at target folder {}", l.to_string_lossy());
    }

    if let Some(l) = target_folder.as_ref() {
        println!("Watching source at  {}", l.to_string_lossy());
    }

    println!("Compiling with features: {}", features.join(", "));

    set_envs()?;

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
            println!("Checking package name: {package} - {pkg}");
            if pkg != package.as_str() {
                return None;
            }
        }
        pkg.targets
            .iter()
            .find(|p| {
                let result = p.crate_types.contains(&String::from("dylib"));
                println!(
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

    if !target_path.ends_with("debug") {
        target_path.push("debug");
    }

    let target_deps_path = target_path.join("deps");

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

    let cmd_string = print_command(&rustc);

    println!("Run rustc {rustc:#?} - {cmd_string}");

    let rustc_output = rustc.output()?;
    let output = std::str::from_utf8(&rustc_output.stdout)?;
    let errout = std::str::from_utf8(&rustc_output.stderr)?;

    if !rustc_output.status.success() {
        bail!("rustc status {:#?}\n{errout}", rustc_output.status);
    }

    println!("rustc output {output}");
    println!("rustc err {errout}");

    let paths = output
        .lines()
        .chain(errout.lines())
        .filter(|v| !v.contains("Compiling ") && !v.contains("Finished "))
        .map(PathBuf::from)
        .chain([target_path.clone(), target_deps_path])
        .collect::<BTreeSet<_>>();

    println!("Paths found {paths:?}");

    let lib_file_name = paths
        .iter()
        .find(|p| {
            p.extension()
                .filter(|p| p.to_string_lossy() != "rlib")
                .is_some()
        })
        .cloned()
        .ok_or(anyhow::Error::msg("No file name for lib"))?;

    let lib_path = target_path.join(lib_file_name);

    // SET ENVIRONMENT VARS HERE!
    let dylib_path_var = cargo_path_utils::dylib_path_envvar();
    let mut env_paths = cargo_path_utils::dylib_path();
    let paths = paths
        .into_iter()
        .filter(|v| v.extension().is_none() && v.is_absolute())
        .collect::<Vec<_>>();

    println!("Filtered paths {paths:?}");

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

        println!(
            "Environment Variables Set {:?}",
            std::env::var(dylib_path_var)
        );

        let settings = BuildSettings {
            lib_path,
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
            manifest: metadata
                .workspace_root
                .into_std_path_buf()
                .join("Cargo.toml"),
            package: pkg.name.clone(),
            features: features.into_iter().collect::<Vec<_>>().join(","),
            target_folder: target_folder.as_ref().cloned().map(|mut v| {
                if v.ends_with("debug") {
                    v.pop();
                }
                v
            }),
            out_folders: paths,
        };

        let settings = settings.to_string();

        println!("Setting DEXTEROUS_BUILD_SETTINGS env to {settings}");
        std::env::set_var("DEXTEROUS_BUILD_SETTINGS", &settings);

        return Ok(BuildSettingsReady::RequiredEnvChange(
            dylib_path_var.to_string(),
            os_paths.to_string_lossy().to_string(),
        ));
    }

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
        manifest: metadata
            .workspace_root
            .into_std_path_buf()
            .join("Cargo.toml"),
        package: pkg.name.clone(),
        features: features.into_iter().collect::<Vec<_>>().join(","),
        target_folder: target_folder.as_ref().cloned().map(|mut v| {
            if v.ends_with("debug") {
                v.pop();
            }
            v
        }),
        out_folders: paths,
    };

    BUILD_SETTINGS
        .set(settings)
        .map_err(|_| Error::msg("Build settings already set"))?;

    println!("Finished Setting Up");

    Ok(BuildSettingsReady::LibraryPath(LibPathSet::new(lib_path)))
}

pub(crate) fn load_build_settings(settings: String) -> anyhow::Result<LibPathSet> {
    let settings = BuildSettings::try_from(settings.as_str())?;
    let lib_path = settings.lib_path.clone();
    BUILD_SETTINGS
        .set(settings)
        .map_err(|_| Error::msg("Build settings already set"))?;
    Ok(LibPathSet::new(lib_path))
}

pub(crate) fn first_exec() -> anyhow::Result<()> {
    println!("First Execution");
    rebuild_internal()
}

static WATCHER: Once = Once::new();

pub(crate) fn run_watcher() {
    println!("run watcher called");
    WATCHER.call_once(|| {
        println!("Setting up watcher");
        if let Err(e) = run_watcher_inner() {
            eprintln!("Couldn't set up watcher - {e:?}");
        };
    });
}

fn run_watcher_inner() -> anyhow::Result<()> {
    let delay = Duration::from_secs(2);
    let Some(BuildSettings { watch_folder, .. }) = BUILD_SETTINGS.get() else {
        bail!("Couldn't get settings...");
    };
    let (watching_tx, watching_rx) = mpsc::channel::<()>();

    println!("Setting up watcher with {watch_folder:?}");
    thread::spawn(move || {
        let (tx, rx) = mpsc::channel();

        println!("Spawned watch thread");
        let debounced = EventDebouncer::new(delay, move |_data: ()| {
            println!("Files Changed");
            let _ = tx.send(());
        });
        println!("Debouncer set up with delay {delay:?}");

        let Ok(mut watcher) = notify::recommended_watcher(move |_| {
            println!("Got Watch Event");
            debounced.put(());
        }) else {
            eprintln!("Couldn't setup watcher");
            return;
        };
        println!("Watcher response set up");

        if let Err(e) = watcher.watch(watch_folder, RecursiveMode::Recursive) {
            eprintln!("Error watching files: {e:?}");
            return;
        }

        watching_tx.send(()).expect("Couldn't send watch");

        while rx.recv().is_ok() {
            rebuild();
        }
    });
    watching_rx.recv()?;
    println!("Watching...");
    Ok(())
}

fn rebuild() {
    if let Err(e) = rebuild_internal() {
        eprintln!("Failed to rebuild: {e:?}");
    }
}

fn rebuild_internal() -> anyhow::Result<()> {
    let Some(BuildSettings {
        manifest,
        features,
        package,
        target_folder,
        out_folders,
        ..
    }) = BUILD_SETTINGS.get()
    else {
        bail!("Couldn't get settings...");
    };

    if let Some(target) = target_folder {
        std::env::set_var("CARGO_BUILD_TARGET_DIR", target.as_os_str());
    }

    let result = Command::new("cargo")
        .env_remove("LD_DEBUG")
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest.as_os_str())
        .arg("-p")
        .arg(package.as_str())
        .arg("--lib")
        .arg("--features")
        .arg(features)
        .status()?;
    // bail!("just want to see result");

    // let mut command = Command::new("cargo");
    // command
    //     .arg("build")
    //     .arg("--manifest-path")
    //     .arg(manifest.as_os_str())
    //     .arg("-p")
    //     .arg(package.as_str())
    //     .arg("--lib")
    //     .arg("--features")
    //     .arg(features);

    // println!("Executing cargo command: {}", print_command(&command));

    // let result = command.status()?;

    if result.success() {
        println!("Build completed");
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
