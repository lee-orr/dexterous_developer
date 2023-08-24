use std::{
    collections::BTreeSet,
    path::PathBuf,
    rc::Rc,
    sync::{mpsc, Once, OnceLock},
    thread,
    time::Duration,
};

use anyhow::{bail, Context};
use cargo::{
    core::{compiler::CompileKind, FeatureValue},
    ops::{CompileFilter, CompileOptions, FilterRule, Packages},
    util::command_prelude::CompileMode,
    Config,
};
use debounce::EventDebouncer;
use notify::{RecursiveMode, Watcher};

struct BuildSettings {
    watch_folder: PathBuf,
    manifest: PathBuf,
    features: BTreeSet<FeatureValue>,
    spect: Packages,
    filter: CompileFilter,
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

pub(crate) fn first_exec(
    package: &Option<String>,
    library: &Option<String>,
    watch: &Option<PathBuf>,
    features: &[String],
) -> anyhow::Result<()> {
    if let Some(p) = package.as_ref() {
        println!("Looking for package {p}");
    }
    if let Some(l) = library.as_ref() {
        println!("Looking for library {l}");
    }

    set_envs();

    let mut manifest = std::env::current_dir().context("Couldn't get current directory")?;
    manifest.push("Cargo.toml");
    let mut config = Config::default().context("Couldn't setup initial config")?;
    config.nightly_features_allowed = true;

    let features = features
        .iter()
        .chain(
            [
                "bevy/dynamic_linking".to_string(),
                "dexterous_developer/hot_internal".to_string(),
            ]
            .iter(),
        )
        .map(|v| FeatureValue::new(v.into()))
        .collect();

    let mut options = CompileOptions::new(&config, CompileMode::Build)
        .context("Couldn't create initial options")?;

    options.cli_features.features = Rc::new(features);

    let ws = cargo::core::Workspace::new(&manifest, &config).context("Couldn't open workspace")?;

    let packages = ws.members();

    let libs = packages.filter_map(|pkg| {
        if let Some(package) = package.as_ref() {
            let pkg = pkg.name();
            println!("Checking package name: {package} - {pkg}");
            if pkg != package.as_str() {
                return None;
            }
        }
        pkg.targets()
            .iter()
            .find(|p| {
                let result = p.is_dylib() || p.is_cdylib();
                println!("Checking {} @ {} - {result}", p.name(), pkg.name());
                result
            })
            .map(|p| (pkg, p))
    });

    let libs: Vec<_> = if let Some(library) = library.as_ref() {
        libs.filter(|v| v.1.name() == library.as_str()).collect()
    } else {
        libs.collect()
    };

    if libs.len() > 1 {
        bail!("Workspace contains multiple libraries - please set the one you want with the --package option");
    }

    let Some((pkg, _lib)) = libs.first() else {
        bail!("Workspace contains no matching libraries");
    };

    options.spec = Packages::Packages(vec![pkg.name().to_string()]);

    options.filter = CompileFilter::new(
        cargo::ops::LibRule::True,
        FilterRule::none(),
        FilterRule::none(),
        FilterRule::none(),
        FilterRule::none(),
    );

    let watch_folder = if let Some(watch) = watch.as_ref() {
        watch.clone()
    } else {
        let mut path = pkg.root().to_path_buf();
        path.push("src");
        path
    };

    BUILD_SETTINGS
        .set(BuildSettings {
            manifest,
            features: options.cli_features.features.as_ref().clone(),
            spect: options.spec.clone(),
            filter: options.filter.clone(),
            watch_folder,
        })
        .map_err(|_| {
            anyhow::Error::msg(
                "Failed to set build settings - likely called first exec multiple times",
            )
        })?;

    options
        .build_config
        .single_requested_kind()
        .context("Couldn't request single compilation")?;

    let compile = cargo::ops::compile(&ws, &options).context("Couldn't compile")?;
    let process_builder = compile
        .target_process("", CompileKind::Host, pkg, None)
        .context("Couldn't build env variables")?;

    for (var, val) in process_builder.get_envs().iter() {
        if let Some(val) = val {
            std::env::set_var(var, val);
        }
    }

    Ok(())
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
        spect,
        filter,
        ..
    }) = BUILD_SETTINGS.get()
    else {
        bail!("Couldn't get settings...");
    };

    set_envs();
    let mut config = Config::default().context("Couldn't setup initial config")?;
    config.nightly_features_allowed = true;
    let ws = cargo::core::Workspace::new(manifest, &config).context("Couldn't open workspace")?;

    let mut options = CompileOptions::new(&config, CompileMode::Build)
        .context("Couldn't create initial options")?;

    options.cli_features.features = Rc::new(features.clone());
    options.spec = spect.clone();
    options.filter = filter.clone();

    options
        .build_config
        .single_requested_kind()
        .context("Couldn't request single compilation")?;

    let _compile = cargo::ops::compile(&ws, &options).context("Couldn't compile")?;

    Ok(())
}
