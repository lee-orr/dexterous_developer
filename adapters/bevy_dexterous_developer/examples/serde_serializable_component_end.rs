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

#[derive(Component, Debug, Serialize, Deserialize)]
#[serde(default)]
struct MySerializableComponent {
    first_field: String,
    second_field: String,
}

impl SerializableType for MySerializableComponent {
    fn get_type_name() -> &'static str {
        "MySerializableComponent"
    }
}

impl Default for MySerializableComponent {
    fn default() -> Self {
        Self {
            first_field: "My First Field".to_string(),
            second_field: "?".to_string()
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

fn update(res : Query<&MySerializableComponent>) {
    let mut list = res.iter().map(|component| {
        format!("{}_{}", component.first_field, component.second_field)
    }).collect::<Vec<_>>();
    list.sort();
    let value = list.join(" - ");
    println!("{value}");
}

fn startup(mut commands : Commands) {
    println!("Press Enter to Progress, or type 'exit' to exit");
    commands.spawn(MySerializableComponent { first_field: "a".to_string(), second_field: "!".to_string()});
    commands.spawn(MySerializableComponent { first_field: "b".to_string(), second_field: "!".to_string()});
}

reloadable_scope!(reloadable(app) {
    app
        .register_serializable_component::<MySerializableComponent>()
        .add_systems(Startup, startup)
        .add_systems(Update, update);
});
