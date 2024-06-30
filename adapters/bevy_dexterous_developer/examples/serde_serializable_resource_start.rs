use std::num::NonZero;

use bevy::{
    app::{AppExit, Startup, Update},
    prelude::*,
    MinimalPlugins,
};
use bevy_dexterous_developer::*;
use serde::{Deserialize, Serialize};

fn terminal_runner(mut app: App) -> AppExit {
    app.update();
    eprintln!("Ready for Input");
    for line in std::io::stdin().lines() {
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            println!("Exiting");
            return AppExit::Success;
        }
        app.update();
    }
    AppExit::Error(NonZero::<u8>::new(1).unwrap())
}

#[derive(Resource, Serialize, Deserialize, Debug)]
struct MySerializableResource {
    first_field: String
}

impl SerializableType for MySerializableResource {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }
}

impl Default for MySerializableResource {
    fn default() -> Self {
        Self {
            first_field: "My Serializable Field".to_string(),
        }
    }
}

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<reloadable>()
        .run();
});

fn update(res : Res<MySerializableResource>) {
    println!("{}", res.first_field);
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

reloadable_scope!(reloadable(app) {
    app
        .init_serializable_resource::<MySerializableResource>()
        .add_systems(Startup, startup)
        .add_systems(Update, update);
});
