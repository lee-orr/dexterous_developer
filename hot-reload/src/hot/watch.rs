use std::thread;

use crate::hot::lib_set::LibPathSet;

pub struct EndWatch;

pub fn run_watcher(
    end_cargo_watch_rx: crossbeam::channel::Receiver<EndWatch>,
    library_paths: &LibPathSet,
) {
    let watch_folder = library_paths.watch_folder.clone();
    let folder = library_paths.folder.clone();
    thread::spawn(move || {
        println!("Spawned watch thread");
        println!("Watch Thread: {:?}", std::thread::current().id());
        let build_cmd = format!(
            "build --lib --target-dir {} --features bevy/dynamic_linking",
            folder.parent().unwrap().to_string_lossy()
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
