pub mod shared;
mod update;
use bevy::{prelude::App, MinimalPlugins};
use bevy_dexterous_developer::{reloadable_main, ReloadableElementsSetup};

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

reloadable_main!(bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<update::reloadable>()
        .init_resource::<shared::StdInput>()
        .init_state::<AppState>()
        .run();
});
