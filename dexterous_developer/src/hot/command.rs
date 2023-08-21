use std::{sync::Once, thread};

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

static WATCHER: Once = Once::new();

pub fn run_watcher(library_paths: &LibPathSet, build_command: &str) {
    WATCHER.call_once(|| {
        run_watcher_inner(library_paths, build_command);
    });
}
fn run_watcher_inner(library_paths: &LibPathSet, build_command: &str) {
    let watch_folder = library_paths.watch_folder.clone();
    let build_command = build_command.to_string();
    println!("Setting up watcher with {watch_folder:?}, build_command: {build_command:?}");
    thread::spawn(move || {
        println!("Spawned watch thread");
        println!("Watch Thread: {:?}", std::thread::current().id());
        let mut cmd = std::process::Command::new("cargo");

        cmd.arg("watch")
            .arg("--watch-when-idle")
            .arg("-w")
            .arg(watch_folder.as_os_str())
            .arg("-x")
            .arg(build_command);
        println!("Spawning command: {cmd:?}");

        let _h = cmd
            .spawn()
            .expect("cargo watch command failed, make sure cargo watch is installed");
        println!("spawned watcher");
    });
}
