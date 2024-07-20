//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily derived from Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use std::{collections::HashSet, time::SystemTime};

use camino::Utf8PathBuf;
use dexterous_developer_builder::incremental_builder::IncrementalRunParams;
use futures_util::future::join_all;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().collect::<Vec<_>>();

    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let lib_drectories = std::env::var("DEXTEROUS_DEVELOPER_LIB_DIRECTORES")?;
    let lib_directories: Vec<Utf8PathBuf> = serde_json::from_str(&lib_drectories)?;
    let framework_directories = std::env::var("DEXTEROUS_DEVELOPER_FRAMEWORK_DIRECTORES")?;
    let framework_directories: Vec<Utf8PathBuf> = serde_json::from_str(&framework_directories)?;
    let zig_path: Utf8PathBuf = Utf8PathBuf::from(std::env::var("ZIG_PATH")?);


    let args = filter_arguments(&target, &args);

    let output_name = {
        let mut next_is_output = false;
        args.iter()
            .find(|arg| {
                if arg.as_str() == "-o" {
                    next_is_output = true;
                } else if next_is_output {
                    return true;
                }
                false
            })
            .cloned()
            .unwrap_or_default()
    };


    let mut dirs = vec![];

    for dir in lib_directories.iter().rev() {
        dirs.push("-L".to_string());
        dirs.push(dir.to_string());
    }

    for dir in framework_directories.iter() {
        dirs.push("-F".to_string());
        dirs.push(dir.to_string());
    }

    let args = dirs.into_iter().chain(args.into_iter()).collect::<Vec<_>>();

    if !output_name.contains(&package_name) {
        eprintln!("Linking Non-Main File - {output_name}");
        let output = tokio::process::Command::new(&zig_path)
            .arg("cc")
            .arg("-target")
            .arg(target)
            .args(&args)
            .spawn()?
            .wait_with_output()
            .await?;

        if !output.status.success() {
            eprintln!("Failed Linking Non Main - {}", args.join(" "));
        }
        std::process::exit(output.status.code().unwrap_or_default());
    }

    let mut next_is_output = false;

    let args = args
        .into_iter()
        .filter(|v| !(v.contains("--gc-sections") || v.contains("-pie")))
        .filter(|v| {
            if v == "-o" {
                next_is_output = true;
                false
            } else if next_is_output {
                next_is_output = true;
                false
            } else {
                true
            }
        })
        .collect::<Vec<String>>();

    let incremental_run_params: IncrementalRunParams =
        serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_INCREMENTAL_RUN")?)?;

    match incremental_run_params {
        IncrementalRunParams::InitialRun => {
            basic_link(
                zig_path,
                args,
                output_file,
                lib_directories,
                framework_directories,
                target,
            )
            .await
        }
        IncrementalRunParams::Patch {
            timestamp,
            previous_versions,
            id,
        } => {
            patch_link(
                zig_path,
                args,
                timestamp,
                previous_versions,
                lib_directories,
                output_file,
                framework_directories,
                target,
                id,
            )
            .await
        }
    }
}

async fn basic_link(
    zig_path: Utf8PathBuf,
    args: Vec<String>,
    output_file: String,
    lib_directories: Vec<Utf8PathBuf>,
    framework_directories: Vec<Utf8PathBuf>,
    target: String,
) -> anyhow::Result<()> {
    let path = Utf8PathBuf::from(output_file);
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }

    let output = tokio::process::Command::new(&zig_path)
        .arg("cc")
        .arg("-target")
        .arg(&target)
        .arg("-fPIC")
        .args(&args)
        .arg("-o")
        .arg(&path)
        .arg("-shared")
        .arg("-rdynamic")
        .spawn()?
        .wait_with_output()
        .await?;
    if !output.status.success() {
        eprintln!("Failed Link Parameters - initial:\nzig cc -target {target} -fPIC {} -o {path} -shared -rdynamic", args.join(" "));
    }
    std::process::exit(output.status.code().unwrap_or_default());
}

async fn patch_link(
    zig_path: Utf8PathBuf,
    args: Vec<String>,
    timestamp: SystemTime,
    previous_versions: Vec<String>,
    lib_directories: Vec<Utf8PathBuf>,
    output_file: String,
    framework_directories: Vec<Utf8PathBuf>,
    target: String,
    id: u32,
) -> anyhow::Result<()> {
    let timestamp = timestamp.duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let mut object_files: Vec<String> = vec![];
    let mut next_is_arch = false;
    let mut arch = None;
    let mut include_args: Vec<String> = vec![];

    for arg in args {
        if next_is_arch {
            arch = Some(arg);
            next_is_arch = false;
        } else if arg == "-arch" {
            next_is_arch = true;
        } else if arg.ends_with(".o") && !arg.contains("symbols.o") {
            object_files.push(arg);
        } else if arg.contains('=') || arg.starts_with("-l") {
            include_args.push(arg);
        } else if arg.contains("rustup/toolchains") {
            include_args.push("-L".to_string());
            include_args.push(arg);
        }
    }

    let output_file = Utf8PathBuf::from(output_file);
    if output_file.exists() {
        tokio::fs::remove_file(&output_file).await?;
    }

    let object_files = join_all(
        object_files
            .into_iter()
            .map(|path| filter_new_paths(path, timestamp)),
    )
    .await
    .into_iter()
    .collect::<anyhow::Result<Vec<_>>>()?;
    let object_files = object_files.into_iter().flatten().collect::<Vec<_>>();

    if object_files.is_empty() {
        eprintln!("No Object Files Changed");
        std::process::exit(0);
    }

    let mut cc = tokio::process::Command::new(&zig_path);

    let mut args = vec!["cc".to_string(), "-target".to_string(), target.clone()];

    if target.contains("mac") {
        args.push("-undefined".to_string());
        args.push("dynamic_lookup".to_string());
        args.push("-dylib".to_string());
        args.push("-shared".to_string());
        args.push("-rdynamic".to_string());
    } else {
        args.push("-shared".to_string());
        args.push("-rdynamic".to_string());
        args.push("-fvisibility=default".to_string());
    }

    args.push("-nodefaultlibs".to_string());
    args.push("-fPIC".to_string());
    args.push("-o".to_string());
    args.push(output_file.to_string());

    if let Some(arch) = &arch {
        args.push("-arch".to_string());
        args.push(arch.clone());
    }

    for name in previous_versions.iter().rev() {
        if !name.ends_with(&format!(".{id}")) {
            args.push(format!("-l{name}"));
        }
    }

    for file in &object_files {
        args.push(file.clone());
    }

    let output = cc.args(&args).spawn()?.wait_with_output().await?;
    if !output.status.success() {
        eprintln!("Failed Link Parameters {id}:\nzig {}", args.join(" "));
    }
    std::process::exit(output.status.code().unwrap_or_default());
}

async fn filter_new_paths(path: String, _timestamp: u64) -> anyhow::Result<Option<String>> {
    Ok(Some(path))
}

fn filter_arguments(target: &str, args: &[String]) -> Vec<String> {
    let windows = target.contains("windows");
    let arm = target.contains("arm");
    let aarch = target.contains("aarch64");
    let mac = target.contains("macos");
    
    args
        .into_iter()
        .filter(|v| {
            !v.contains("dexterous_developer_incremental_linker")
                && !v.contains("incremental_c_compiler")
                && UNSUPPORTED_ZIG_ARGS.iter().find(|arg| v.contains(**arg)).is_none()
        })
        .filter_map(|v| {if v == "-lgcc_s" {
                Some("-lunwind".to_owned())
            } else if (windows || arm) && v.contains("libcompiler_builtins-") {
                None
            }  else if windows {
                if v.contains("-Bdynamic") {
                    Some("-Wl,-search_paths_first".to_owned())
                } else if v == "-lgcc_eh"  {
                    Some("-lc++".to_string())
                } else {
                    Some(v.clone())
                }
            } else {
                Some(v.clone())
            }
        })
        .collect::<Vec<_>>()
}

const UNSUPPORTED_ZIG_ARGS : [&'static str;10] = [
    "--target",
    "-lwindows",
    "-l:libpthread.a",
    "--disable-auto-image-base",
    "--dynamicbase",
    "--large-address-aware",
    "list.def",
    "--no-undefined-version",
    "-dylib",
    "-exported_symbols_list"
];