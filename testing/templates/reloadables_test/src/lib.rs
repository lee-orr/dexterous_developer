pub mod shared;
mod update;
use bevy::{prelude::App, MinimalPlugins};
use dexterous_developer::*;

use crate::shared::AppState;

fn terminal_runner(mut app: App) {
    app.update();
    for line in std::io::stdin().lines() {
        println!("Runner Got {line:?}");
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            println!("Exiting");
            return;
        }
        app.insert_resource(shared::StdInput(typed));

        println!("Running Update");
        app.update();
        println!("Update Ended");
    }
}

#[bevy_app_setup]
pub fn bevy_main(initial_plugins: InitializeApp) {
    initial_plugins
        .initialize::<MinimalPlugins>()
        .modify_fence(|app| {
            app.init_resource::<shared::StdInput>();
        })
        .sync_resource_from_fence::<shared::StdInput>()
        .app_with_runner(terminal_runner)
        .add_plugins(update::MyPlugin)
        .add_state::<AppState>();
}
