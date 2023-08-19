use std::{process::ExitStatus, thread};

use crate::internal_shared::lib_path_set::LibPathSet;

pub fn run_watcher(library_paths: &LibPathSet, build_command: &str) {
    let watch_folder = library_paths.watch_folder.clone();
    let build_command = build_command.to_string();
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

        let mut h = cmd
            .spawn()
            .expect("cargo watch command failed, make sure cargo watch is installed");
        println!("spawned watcher");
    });
}
