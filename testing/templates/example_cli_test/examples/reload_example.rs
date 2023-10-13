use bevy::{prelude::App, MinimalPlugins};
use dexterous_developer::{hot_bevy_main, InitialPlugins, ReloadableElementsSetup};
use lib_simple_cli_test::*;

#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<update::reloadable>()
        .run();
}
