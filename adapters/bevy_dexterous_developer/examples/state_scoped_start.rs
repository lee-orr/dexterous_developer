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

#[derive(States, Debug, Default, Hash, PartialEq, Eq, Clone)]
enum MyState {
    #[default]
    Initial,
    Another,
}

impl ReplacableType for MyState {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        let value = match self {
            MyState::Initial => [0],
            MyState::Another => [1],
        };
        Ok(value.to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        let value = if let Some(val) = val.first() {
            if *val == 1 {
                MyState::Another
            } else {
                MyState::Initial
            }
        } else {
            MyState::Initial
        };
        Ok(value)
    }
}

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<reloadable>()
        .run();
});

fn set_next_state(mut next_state: ResMut<NextState<MyState>>) {
    next_state.set(MyState::Another);
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

#[derive(Component, Debug, Serialize, Deserialize)]
struct MySerializableComponent {
    first_field: String,
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
        }
    }
}

fn update(state: Res<State<MyState>>, query: Query<&MySerializableComponent>) {
    let mut query = query
        .iter()
        .map(|v| v.first_field.clone())
        .collect::<Vec<_>>();
    query.sort();

    let state = match state.get() {
        MyState::Initial => 0,
        MyState::Another => 1,
    };

    let list = query.join("");

    println!("{state} - {list}.");
}

reloadable_scope!(reloadable(app) {
    app
        .add_systems(Startup, startup)
        .add_systems(Update, set_next_state.run_if(in_state(MyState::Initial)))
        .add_systems(Update, update)
        .reset_setup::<MySerializableComponent, _>(|mut commands: Commands| {
            commands.spawn(MySerializableComponent {
                first_field: "a".to_string()
            });
            commands.spawn((MySerializableComponent {
                first_field: "b".to_string()
            }, StateScoped(MyState::Initial)));
        })
        .init_state::<MyState>()
        .enable_state_scoped_entities::<MyState>();
});
