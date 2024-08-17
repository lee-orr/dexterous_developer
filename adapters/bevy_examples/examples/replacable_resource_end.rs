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

#[derive(Resource, Debug)]
struct MySerializableResource {
    first_field: String,
    second_field: String,
}

impl ReplacableType for MySerializableResource {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        let value = format!("{}::{}", self.first_field, self.second_field);
        Ok(value.as_bytes().to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        let value = std::str::from_utf8(val)?;
        let mut split = value.split("::");
        let first_field = split
            .next()
            .map(|v| v.to_string())
            .unwrap_or("No First Field".to_string());
        let second_field = split
            .next()
            .map(|v| v.to_string())
            .unwrap_or("Missing Second Field".to_string());
        Ok(MySerializableResource {
            first_field,
            second_field,
        })
    }
}

impl Default for MySerializableResource {
    fn default() -> Self {
        Self {
            first_field: "My First Field".to_string(),
            second_field: "My New Field".to_string(),
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

fn update(res: Res<MySerializableResource>) {
    println!("{} - {}", res.first_field, res.second_field);
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
