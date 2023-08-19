use std::{process::ExitStatus, thread};

use crate::internal_shared::lib_path_set::LibPathSet;

pub fn create_build_command(library_paths: &LibPathSet, features: &[String]) -> String {
    let folder = library_paths.folder.clone();
    let package = library_paths.package.clone();
    let features = features
        .iter()
        .map(|v| format!("--features {v}"))
        .collect::<Vec<String>>()
        .join(" ");
    format!(
        "build -p {package} --lib --target-dir {} --features bevy/dynamic_linking --features dexterous_developer/hot_internal {features}",
        folder.parent().unwrap().to_string_lossy(),
    )
}

pub fn first_exec(build_command: &str) -> std::io::Result<()> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.args(build_command.split_whitespace());

    println!("Creating initial build: {cmd:?}");

    let _ = cmd.status()?;
    Ok(())
}
