pub mod update;
use bevy::{prelude::App};


pub fn terminal_runner(mut app: App) {
    app.update();
    for line in std::io::stdin().lines() {
        println!("Runner Got {line:?}");
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            println!("Exiting");
            return;
        }
        println!("Running Update");
        app.update();
        println!("Update Ended");
    }
}
