mod update;
use bevy::{prelude::App, MinimalPlugins};
use dexterous_developer::{hot_bevy_main, InitialPlugins, ReloadableElementsSetup};

fn terminal_runner(mut app: App) {
    println!("Press Enter to Progress, or type 'exit' to exit");
    for line in std::io::stdin().lines() {
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            return;
        }
        app.update();
    }
}

#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<update::reloadable>()
        .run();
}
