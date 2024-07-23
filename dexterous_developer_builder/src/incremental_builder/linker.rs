//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily derived from Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use std::time::SystemTime;

use super::builder::IncrementalRunParams;
use camino::Utf8PathBuf;
use cargo_zigbuild::Zig;
use futures_util::future::join_all;
use tokio::io::AsyncWriteExt;

pub async fn linker() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    let args = args.collect::<Vec<_>>();

    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let lib_drectories = std::env::var("DEXTEROUS_DEVELOPER_LIB_DIRECTORES")?;
    let lib_directories: Vec<Utf8PathBuf> = serde_json::from_str(&lib_drectories)?;

    let args = adjust_arguments(&target, &args).await?;

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

    let args = dirs.into_iter().chain(args.into_iter()).collect::<Vec<_>>();

    if !output_name.contains(&package_name) {
        eprintln!("Linking Non-Main File - {output_name}\n{}", args.join(" "));
        let zig = Zig::Cc { args: args.clone() };

        if let Err(e) = zig.execute() {
            eprintln!("Failed Linking Non Main - {e}\n{}", args.join(" "));
            std::process::exit(1);
        }
        std::process::exit(0);
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
                next_is_output = false;
                false
            } else {
                true
            }
        })
        .collect::<Vec<String>>();

    let incremental_run_params: IncrementalRunParams =
        serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_INCREMENTAL_RUN")?)?;

    match incremental_run_params {
        IncrementalRunParams::InitialRun => basic_link(args, output_file).await,
        IncrementalRunParams::Patch {
            timestamp,
            previous_versions,
            id,
        } => patch_link(args, timestamp, previous_versions, output_file, target, id).await,
    }
}

async fn basic_link(args: Vec<String>, output_file: String) -> anyhow::Result<()> {
    let path = Utf8PathBuf::from(output_file);
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }

    let args = vec![
        args.iter().map(|v| v.as_str()).collect(),
        vec!["-o", path.as_str(), "-shared", "-rdynamic"],
    ]
    .into_iter()
    .flatten()
    .map(|v| v.to_string())
    .collect::<Vec<_>>();

    eprintln!("Initial Build");

    let zig = Zig::Cc { args: args.clone() };

    if let Err(e) = zig.execute() {
        eprintln!("Failed Linking - {e}\n{}", args.join(" "));
        std::process::exit(1);
    }
    std::process::exit(0);
}

async fn patch_link(
    args: Vec<String>,
    timestamp: SystemTime,
    previous_versions: Vec<String>,
    output_file: String,
    target: String,
    id: u32,
) -> anyhow::Result<()> {
    let timestamp = timestamp.duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let mut object_files: Vec<String> = vec![];
    let mut arch = None;
    let mut include_args: Vec<String> = vec![];

    let mut arg_iter = args.iter();

    while let Some(arg) = arg_iter.next() {
        if *arg == "-arch" {
            arch = arg_iter.next().cloned();
        } else if arg == "-L" {
            if let Some(arg) = arg_iter.next() {
                include_args.push("-L".to_string());
                include_args.push(arg.clone());
            }
        } else if arg.contains('=') || arg.starts_with("-l") {
            include_args.push(arg.clone());
        } else if arg.ends_with(".o") && !arg.contains("symbols.o") {
            object_files.push(arg.clone());
        } else if arg == "-target" {
            if let Some(arg) = arg_iter.next() {
                include_args.push("-target".to_string());
                include_args.push(arg.clone());
            }
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

    let mut args = include_args;

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

    let zig = Zig::Cc { args: args.clone() };

    if let Err(output) = zig.execute() {
        eprintln!(
            "Failed Link Parameters {id} - {output}:\n {}",
            args.join(" ")
        );
        std::process::exit(1);
    }
    std::process::exit(0);
}

async fn filter_new_paths(path: String, _timestamp: u64) -> anyhow::Result<Option<String>> {
    Ok(Some(path))
}

async fn adjust_arguments(target: &str, args: &[String]) -> anyhow::Result<Vec<String>> {
    let path =  if let Some(file) = args.first() {
        if args.len() == 1 && file.starts_with("@") && file.ends_with("linker-arguments") {
            let path = file.trim_start_matches("@");
            let path = Utf8PathBuf::from(path);
            if path.exists() {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    }  else {
        None
    };

    let mut args = if let Some(path) = &path {
        let file = tokio::fs::read_to_string(&path).await?;
        file.lines().map(|v| v.to_owned()).collect()
    } else {
        args.iter()
            .filter(|v| {
                !v.contains("dexterous_developer_incremental_linker")
                    && !v.contains("incremental_c_compiler")
            })
            .cloned()
            .collect::<Vec<_>>()
    };


    let has_target = args.iter().find(|v| v.contains("-target")).is_some();

    if !has_target {
        args.push("-target".to_string());
        args.push(target.to_string());
    }

    if let Some(path) = &path {
        tokio::fs::remove_file(&path).await?;
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(args.join("\n").as_bytes()).await?;
        Ok(vec![format!("@{}", Utf8PathBuf::from_path_buf(dunce::canonicalize(&path)?).map_err(|v| anyhow::anyhow!("{v:?}"))?)])
    } else {
        Ok(args)
    }
}
