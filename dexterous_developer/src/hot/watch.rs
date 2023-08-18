use std::thread;

use crate::internal_shared::lib_path_set::LibPathSet;

pub struct EndWatch;

pub fn run_watcher(
    end_cargo_watch_rx: crossbeam::channel::Receiver<EndWatch>,
    library_paths: &LibPathSet,
    features: &[String],
) {
    let watch_folder = library_paths.watch_folder.clone();
    let folder = library_paths.folder.clone();
    let package = library_paths.package.clone();
    let features = features
        .iter()
        .map(|v| format!("--features {v}"))
        .collect::<Vec<String>>()
        .join(" ");
    thread::spawn(move || {
        println!("Spawned watch thread");
        println!("Watch Thread: {:?}", std::thread::current().id());
        let build_cmd = format!(
            "build -p {package} --lib --target-dir {} --features bevy/dynamic_linking --features dexterous_developer/hot_internal {features}",
            folder.parent().unwrap().to_string_lossy(),
        );

        let mut cmd = std::process::Command::new("cargo");

        cmd.arg("watch")
            .arg("--watch-when-idle")
            .arg("-w")
            .arg(watch_folder.as_os_str())
            .arg("-x")
            .arg(build_cmd);
        println!("Spawning command: {cmd:?}");

        let mut h = cmd
            .spawn()
            .expect("cargo watch command failed, make sure cargo watch is installed");
        println!("spawned watcher");

        let _ = end_cargo_watch_rx.recv();

        println!("Closing out {:?}", h);
        let _ = h.kill();
    });
}
