use std::num::NonZero;

use bevy::{
    app::{AppExit, Startup, Update},
    prelude::*,
    MinimalPlugins,
};
use bevy_dexterous_developer::*;

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

#[derive(Component, Debug)]
struct MySerializableComponent {
    first_field: String,
    second_field: String,
}

impl ReplacableType for MySerializableComponent {
    fn get_type_name() -> &'static str {
        "MySerializableComponent"
    }
    
    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        let value = format!("{}::{}", self.first_field, self.second_field);
        Ok(value.as_bytes().to_vec())
    }
    
    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        let value = std::str::from_utf8(val)?;
        let mut split = value.split("::");
        let first_field = split.next().map(|v| v.to_string()).unwrap_or(format!("No First Field"));
        let second_field = split.next().map(|v| v.to_string()).unwrap_or(format!("?"));
        Ok(Self { first_field, second_field })
    }
}

impl Default for MySerializableComponent {
    fn default() -> Self {
        Self {
            first_field: "My First Field".to_string(),
            second_field: "Second Field".to_string()
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
