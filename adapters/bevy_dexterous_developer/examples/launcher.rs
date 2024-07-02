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
    println!("In Initial State");
    next_state.set(MyState::Another);
}

fn in_another_state() {
    println!("In Another State");
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

fn entered_initial() {
    println!("Entered Initial");
}

fn entered_another() {
    println!("Entered Another");
}

reloadable_scope!(reloadable(app) {
    app
        .add_systems(Startup, startup)
        .add_systems(Update, set_next_state.run_if(in_state(MyState::Initial)))
        .add_systems(Update, in_another_state.run_if(in_state(MyState::Another)))
        .add_systems(OnEnter(MyState::Initial), entered_initial)
        .add_systems(OnEnter(MyState::Another), entered_another)
        .init_state::<MyState>();
});


fn main() {
    bevy_main();
}