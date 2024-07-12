//! A linker that catches incremental change artifacts applies recent changes to a dynamic library
//! 
//! Heavily derived from Jon Kelley's work - https://github.com/jkelleyrtp/ipbp/blob/main/packages/patch-linker/src/main.rs

use std::time::SystemTime;

use camino::Utf8PathBuf;
use dexterous_developer_builder::incremental_builder::IncrementalRunParams;
use futures_util::future::join_all;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = std::env::args().filter(|v| !v.contains("dexterous_developer_incremental_linker")).collect::<Vec<String>>();
    let incremental_run_params : IncrementalRunParams = serde_json::from_str(&std::env::var("DEXTEROUS_DEVELOPER_INCREMENTAL_RUN")?)?;

    

    match incremental_run_params {
        IncrementalRunParams::InitialRun => basic_link(args).await,
        IncrementalRunParams::Patch {id,timestamp, previous_versions, lib_directories } => patch_link(args, id, timestamp, previous_versions, lib_directories).await
    }
}


async fn basic_link(args: Vec<String>) -> anyhow::Result<()> {
    let mut next_is_output = false;
    let mut output_file = None;

    for arg in &args {
        if next_is_output {
            output_file = Some(arg.to_string());
            next_is_output = false;
        } else if arg == "-o" {
            next_is_output = true;
        }
    }

    let Some(output_file) = output_file else {
        panic!("Couldn't determine output file")
    };
    
    let path = Utf8PathBuf::from(output_file);
    if path.exists() {
        tokio::fs::remove_file(&path).await?;
    }

    let output = tokio::process::Command::new("zig").arg("cc").arg("-fPIC").args(&args).spawn()?.wait_with_output().await?;
    std::process::exit(output.status.code().unwrap_or_default());
}

async fn patch_link(args: Vec<String>, id: u32, timestamp: SystemTime, previous_versions: Vec<Utf8PathBuf>, lib_directories: Vec<Utf8PathBuf>) -> anyhow::Result<()> {
    let timestamp = timestamp.duration_since(std::time::UNIX_EPOCH)?.as_secs();
    let mut object_files : Vec<String> = vec![];
    let mut next_is_output = false;
    let mut output_file = None;
    let mut next_is_arch = false;
    let mut arch = None;
    let mut include_args : Vec<String> = vec![];

    for arg in args {
        if next_is_output {
            output_file = Some(arg);
            next_is_output = false;
        } else if next_is_arch {
            arch = Some(arg);
            next_is_arch = false;
        } else if arg == "-o" {
            next_is_output = true;
        } else if arg == "-arch" {
            next_is_arch = true;
        } else if arg.ends_with(".o") && !arg.contains("symbols.o") {
            object_files.push(arg);
        } else if arg.contains("=") {
            include_args.push(arg);
        } else if arg.starts_with("-l") {
            include_args.push(arg);
        } else if arg.contains("rustup/toolchains") {
            include_args.push("-L".to_string());
            include_args.push(arg);
        }
    }

    let Some(output_file) = output_file else {
        panic!("Couldn't determine output file")
    };

    let output_file = Utf8PathBuf::from(output_file);
    if output_file.exists() {
        tokio::fs::remove_file(&output_file).await?;
    }

    let object_files = join_all(object_files.into_iter().map(|path| filter_new_paths(path, timestamp))).await.into_iter().collect::<anyhow::Result<Vec<_>>>()?;
    let object_files = object_files.into_iter().filter_map(|v| v).collect::<Vec<_>>();

    if object_files.len() == 0 {
        eprintln!("No Object Files Changed");
        std::process::exit(0);
    }

    let mut cc = tokio::process::Command::new("zig");

    let mut args = vec!["cc".to_string()];


    args.push("-shared".to_string());
    args.push("-rdynamic".to_string());
    args.push("-fvisibility=default".to_string());
    args.push("-nodefaultlibs".to_string());
    args.push("-fPIC".to_string());
    args.push("-o".to_string());
    args.push(output_file.to_string());

    if let Some(arch) = &arch {
        args.push("-arch".to_string());
        args.push(arch.clone());
    }
    for dir in lib_directories.iter().rev() {
        args.push("-L".to_string());
        args.push(dir.to_string());
    }

    for file in previous_versions.iter().rev() {
        if let Some(filename)= file.file_name() {
            let filename = filename.replacen("lib", "", 1).replace(".so", "");
            args.push(format!("-l{filename}"));
        }
    }

    for file in &object_files {
        args.push(file.clone());
    }

    let output = cc.args(&args).spawn()?.wait_with_output().await?;
    if !output.status.success() {
        eprintln!("Failed Link Parameters:\nzig {}", args.join(" "));
    }
    std::process::exit(output.status.code().unwrap_or_default());
}

async fn filter_new_paths(path: String, _timestamp: u64) -> anyhow::Result<Option<String>> {
    Ok(Some(path))
}