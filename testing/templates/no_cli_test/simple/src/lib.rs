mod update;
use bevy::{prelude::App, MinimalPlugins};
use dexterous_developer::*;

fn terminal_runner(mut app: App) {
    app.update();
    for line in std::io::stdin().lines() {
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

#[bevy_app_setup]
pub fn bevy_main(initial_plugins: InitializeApp) {
    initial_plugins
        .initialize::<MinimalPlugins>()
        .app_with_runner(terminal_runner)
        .add_plugins(update::MyPlugin);
}
