use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::{mpsc, Once, OnceLock},
    thread,
    time::Duration,
};

use anyhow::{bail, Context, Error};
use cargo_metadata::Package;
use debounce::EventDebouncer;
use notify::{RecursiveMode, Watcher};

use crate::{internal_shared::LibPathSet, HotReloadOptions};

struct BuildSettings {
    watch_folder: PathBuf,
    manifest: PathBuf,
    package: String,
    features: String,
    target_folder: Option<PathBuf>,
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
    ("RUSTFLAGS", "-Zshare-generics=y -Clink-arg=-fuse-ld=lld"),
];
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const RUSTC_ARGS: [(&str, &str); 2] = [
    ("RUSTUP_TOOLCHAIN", "nightly"),
    (
        "RUSTFLAGS",
        "-Zshare-generics=y -Clink-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.ll",
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

fn set_envs() {
    for (var, val) in RUSTC_ARGS.iter() {
        std::env::set_var(var, val);
    }
}

pub(crate) fn setup_build_settings(options: &HotReloadOptions) -> anyhow::Result<LibPathSet> {
    let HotReloadOptions {
        manifest_path,
        package,
        lib_name,
        watch_folder,
        target_folder,
        features,
        set_env,
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

    set_envs();

    let features = features
        .iter()
        .cloned()
        .chain(
            [
                "bevy/dynamic_linking".to_string(),
                "dexterous_developer/hot_internal".to_string(),
            ]
            .into_iter(),
        )
        .collect::<BTreeSet<_>>();

    let mut get_metadata = cargo_metadata::MetadataCommand::new();
    get_metadata.no_deps();
    if let Some(manifest) = manifest_path {
        get_metadata.manifest_path(&manifest);
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

    let target_path = if let Some(target) = target_folder {
        target.clone()
    } else {
        metadata.target_directory.into_std_path_buf()
    };

    let target_deps_path = target_path.join("deps");

    let mut rustc = Command::new("rustc");
    rustc
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
        .chain([target_path.clone(), target_deps_path].into_iter())
        .collect::<BTreeSet<_>>();

    println!("Paths found {paths:?}");

    let lib_file_name = paths
        .iter()
        .filter(|p| {
            p.extension()
                .filter(|p| p.to_string_lossy() != "rlib")
                .is_some()
        })
        .next()
        .cloned()
        .ok_or(anyhow::Error::msg("No file name for lib"))?;

    let settings = BuildSettings {
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
            v.pop();
            v
        }),
    };

    BUILD_SETTINGS
        .set(settings)
        .map_err(|_| Error::msg("Build settings already set"))?;

    // SET ENVIRONMENT VARS HERE!
    let dylib_path_var = cargo_path_utils::dylib_path_envvar();
    let mut env_paths = cargo_path_utils::dylib_path();
    let mut paths = paths
        .into_iter()
        .filter(|v| v.is_dir() && v.is_absolute())
        .collect();
    env_paths.append(&mut paths);

    let paths = std::env::join_paths(env_paths)?;

    std::env::set_var(dylib_path_var, paths);

    println!("Finished Setting Up");

    Ok(LibPathSet::new(target_path.join(lib_file_name)))
}

pub(crate) fn first_exec() -> anyhow::Result<()> {
    println!("First Execution");
    rebuild_internal()
}

static WATCHER: Once = Once::new();

pub(crate) fn run_watcher() {
    WATCHER.call_once(|| {
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
        println!("Watching...");

        while rx.recv().is_ok() {
            rebuild();
        }
    });
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
        watch_folder: _,
        package,
        target_folder,
    }) = BUILD_SETTINGS.get()
    else {
        bail!("Couldn't get settings...");
    };

    set_envs();

    if let Some(target) = target_folder {
        std::env::set_var("CARGO_BUILD_TARGET_DIR", target.as_os_str());
    }

    let mut command = Command::new("cargo");
    command
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest.as_os_str())
        .arg("-p")
        .arg(package.as_str())
        .arg("--lib")
        .arg("--features")
        .arg(features);

    println!("Executing cargo command: {}", print_command(&command));

    let result = command.status()?;

    if result.success() {
        println!("Build completed");
    } else {
        bail!(
            "Failed to build: {}
        env:
        {}",
            print_command(&command),
            std::env::vars()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    Ok(())
}

// Copied from cargo repo: https://github.com/rust-lang/cargo/blob/master/crates/cargo-util/src/paths.rs
mod cargo_path_utils {
    use std::{env, path::PathBuf};

    pub(super) fn dylib_path_envvar() -> &'static str {
        if cfg!(windows) {
            "PATH"
        } else if cfg!(target_os = "macos") {
            // When loading and linking a dynamic library or bundle, dlopen
            // searches in LD_LIBRARY_PATH, DYLD_LIBRARY_PATH, PWD, and
            // DYLD_FALLBACK_LIBRARY_PATH.
            // In the Mach-O format, a dynamic library has an "install path."
            // Clients linking against the library record this path, and the
            // dynamic linker, dyld, uses it to locate the library.
            // dyld searches DYLD_LIBRARY_PATH *before* the install path.
            // dyld searches DYLD_FALLBACK_LIBRARY_PATH only if it cannot
            // find the library in the install path.
            // Setting DYLD_LIBRARY_PATH can easily have unintended
            // consequences.
            //
            // Also, DYLD_LIBRARY_PATH appears to have significant performance
            // penalty starting in 10.13. Cargo's testsuite ran more than twice as
            // slow with it on CI.
            "DYLD_FALLBACK_LIBRARY_PATH"
        } else if cfg!(target_os = "aix") {
            "LIBPATH"
        } else {
            "LD_LIBRARY_PATH"
        }
    }

    /// Returns a list of directories that are searched for dynamic libraries.
    ///
    /// Note that some operating systems will have defaults if this is empty that
    /// will need to be dealt with.
    pub(super) fn dylib_path() -> Vec<PathBuf> {
        match env::var_os(dylib_path_envvar()) {
            Some(var) => env::split_paths(&var).collect(),
            None => Vec::new(),
        }
    }
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
