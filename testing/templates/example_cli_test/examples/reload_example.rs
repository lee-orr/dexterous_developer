use bevy::{prelude::App, MinimalPlugins};
use dexterous_developer::*;
use lib_simple_cli_test::*;

#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    initial_plugins
        .initialize::<MinimalPlugins>()
        .app_with_runner(terminal_runner)
        .add_plugins(update::MyPlugin);
}
