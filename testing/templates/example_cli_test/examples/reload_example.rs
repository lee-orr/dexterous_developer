use bevy::{prelude::App, MinimalPlugins};
use bevy_dexterous_developer::{reloadable_main, ReloadableElementsSetup};
use lib_example_cli_test::*;

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<update::reloadable>()
        .run();
});
