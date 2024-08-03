//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//!
//! Heavily derived from Jon Kelley's work - <https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs>

use std::time::SystemTime;

use super::builder::IncrementalRunParams;
use anyhow::{bail, Context};
use camino::Utf8PathBuf;
use futures_util::future::join_all;
use tokio::io::AsyncWriteExt;

pub async fn linker() -> anyhow::Result<()> {
    let mut args = std::env::args();
    args.next();
    let args = args.collect::<Vec<_>>();

    let linker_exec = "cc".to_string();
    let package_name = std::env::var("DEXTEROUS_DEVELOPER_PACKAGE_NAME")?;
    let output_file = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE")?;
    let file_name = std::env::var("DEXTEROUS_DEVELOPER_OUTPUT_FILE_NAME")?;
    let target = std::env::var("DEXTEROUS_DEVELOPER_LINKER_TARGET")?;
    let lib_drectories = std::env::var("DEXTEROUS_DEVELOPER_LIB_DIRECTORES")?;
    let lib_directories: Vec<Utf8PathBuf> = serde_json::from_str(&lib_drectories)?;

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

    let args = adjust_arguments(&target, &args, &lib_directories).await?;

    if !output_name.contains(&package_name) {
        eprintln!("Linking Non-Main File - {output_name}\n{}", args.join(" "));

        let mut zig = tokio::process::Command::new(linker_exec);
        zig.args(args.clone());

        let result = zig.output().await?;

        if !result.status.success() {
            eprintln!("Failed Link Non Main - {output_name}:\n {}", args.join(" "));
            eprintln!(
                "PRINTOUT:\n{}",
                std::str::from_utf8(&result.stderr).unwrap_or_default()
            );
            std::process::exit(1);
        }
        std::process::exit(0);
    }

    let mut next_is_output = false;

    let args = args
        .into_iter()
        .filter(|v| {
            !(v.contains("--gc-sections") || v.contains("-pie") || v.contains("--version-script"))
        })
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
        IncrementalRunParams::InitialRun => {
            basic_link(args, output_file, file_name, linker_exec).await
        }
        IncrementalRunParams::Patch {
            timestamp,
            previous_versions,
            id,
        } => {
            patch_link(
                args,
                timestamp,
                previous_versions,
                output_file,
                file_name,
                target,
                id,
                linker_exec,
            )
            .await
        }
    }
}

async fn basic_link(
    args: Vec<String>,
    output_file: String,
    file_name: String,
    linker_exec: String,
) -> anyhow::Result<()> {
    let path = Utf8PathBuf::from(output_file);
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }

    let mut args = args;

    args.push("-o".to_string());
    args.push(path.to_string());
    args.push(format!("-Wl,-soname,{file_name}"));

    if !args.contains(&"-shared".to_owned()) {
        args.push("-shared".to_owned());
    }

    if !args.contains(&"-rdynamic".to_owned()) {
        args.push("-rdynamic".to_owned());
    }

    eprintln!("\nInitial Build -\n{}", args.join(" "));
    // panic!();
    let mut zig = tokio::process::Command::new(linker_exec);
    zig.args(args.clone());

    let result = zig.output().await?;

    if !result.status.success() {
        eprintln!("Failed Linking Initial:\n {}", args.join(" "));
        eprintln!(
            "PRINTOUT:\n{}",
            std::str::from_utf8(&result.stderr).unwrap_or_default()
        );
        std::process::exit(1);
    }

    std::process::exit(0);
}

async fn patch_link(
    args: Vec<String>,
    timestamp: SystemTime,
    previous_versions: Vec<String>,
    output_file: String,
    file_name: String,
    target: String,
    id: u32,
    linker_exec: String,
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

    if target.contains("apple") {
        args.push("-undefined".to_string());
        args.push("dynamic_lookup".to_string());
        args.push("-dylib".to_string());
        args.push("-shared".to_string());
        args.push("-rdynamic".to_string());
    } else {
        if !args.contains(&"-shared".to_owned()) {
            args.push("-shared".to_string());
            args.push("-rdynamic".to_string());
        }
        args.push("-fvisibility=default".to_string());
    }

    args.push("-nodefaultlibs".to_string());
    args.push("-fPIC".to_string());
    args.push("-o".to_string());
    args.push(output_file.to_string());
    args.push(format!("-Wl,-soname,{file_name}"));

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

    // panic!("ARGS: {}", args.join(" "));

    let mut zig = tokio::process::Command::new(linker_exec);
    zig.args(args.clone());

    let result = zig.output().await?;

    if !result.status.success() {
        eprintln!("Failed Link Parameters {id}:\n {}", args.join(" "));
        eprintln!(
            "PRINTOUT:\n{}",
            std::str::from_utf8(&result.stderr).unwrap_or_default()
        );
        std::process::exit(1);
    }
    std::process::exit(0);
}

async fn filter_new_paths(path: String, _timestamp: u64) -> anyhow::Result<Option<String>> {
    Ok(Some(path))
}

async fn adjust_arguments(
    target: &str,
    args: &[String],
    lib_directories: &[Utf8PathBuf],
) -> anyhow::Result<Vec<String>> {
    let path = if let Some(file) = args.first() {
        if file.starts_with('@') && file.ends_with("linker-arguments") {
            let path = file.trim_start_matches('@');
            let path = Utf8PathBuf::from(path);
            println!("Have the file path");
            if path.exists() {
                Some(path)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let args = if let Some(path) = &path {
        let file = tokio::fs::read(&path).await?;
        let file = if target.contains("msvc") {
            if file[0..2] != [255, 254] {
                bail!(
                    "linker response file `{}` didn't start with a utf16 BOM",
                    &path
                );
            }
            let content_utf16: Vec<u16> = file[2..]
                .chunks_exact(2)
                .map(|a| u16::from_ne_bytes([a[0], a[1]]))
                .collect();
            String::from_utf16(&content_utf16).with_context(|| {
                format!(
                    "linker response file `{}` didn't contain valid utf16 content",
                    &path
                )
            })?
        } else {
            String::from_utf8(file)?
        };
        file.lines().map(|v| v.to_owned()).collect()
    } else {
        args.to_vec()
    };

    let mut new_args = vec![];
    for arg in args.into_iter().filter(|v| {
        !v.contains("dexterous_developer_incremental_linker")
            && !v.contains("incremental_c_compiler")
    }) {
        new_args.push(arg);
    }

    for dir in lib_directories.iter().rev() {
        new_args.push("-L".to_string());
        new_args.push(dir.to_string());
    }

    let args = new_args;

    if let Some(path) = &path {
        tokio::fs::remove_file(&path).await?;
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(args.join("\n").as_bytes()).await?;
        Ok(vec![format!(
            "@{}",
            Utf8PathBuf::from_path_buf(dunce::canonicalize(path)?)
                .map_err(|v| anyhow::anyhow!("{v:?}"))?
        )])
    } else {
        Ok(args)
    }
}
