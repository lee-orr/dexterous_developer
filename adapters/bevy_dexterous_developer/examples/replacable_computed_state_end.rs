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
    InitialState,
    AnotherState,
    ThirdState
}

impl ReplacableType for MyState {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        let value = match self {
            MyState::InitialState => [0],
            MyState::AnotherState => [1],
            MyState::ThirdState => [2]
        };
        Ok(value.to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        let value = if let Some(val) = val.get(0) {
            if *val == 1 {
                MyState::AnotherState
            } else if *val == 2 {
                MyState::ThirdState
            } else {
                MyState::InitialState
            }
        } else {
            MyState::InitialState
        };
        Ok(value)
    }
}


#[derive(Debug, Default, Hash, PartialEq, Eq, Clone)]
struct InAnotherState;

impl ComputedStates for InAnotherState {
    type SourceStates = MyState;

    fn compute(sources: Self::SourceStates) -> Option<Self> {
        match sources {
            MyState::InitialState => None,
            _ => Some(Self),
        }
    }
}

impl ReplacableType for InAnotherState {
    fn get_type_name() -> &'static str {
        "InAnotherState"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        Ok(vec![])
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        Ok(Self)
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
    next_state.set(MyState::AnotherState);
}

fn in_another_state(state: Res<State<MyState>>) {
    let value = match state.get() {
        MyState::InitialState => "1",
        MyState::AnotherState => "2",
        MyState::ThirdState => "3",
    };
    println!("In Another State - {value}");
}

fn next_another_state(mut next_state: ResMut<NextState<MyState>>) {
    next_state.set(MyState::ThirdState);
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}


reloadable_scope!(reloadable(app) {
    app
        .add_systems(Startup, startup)
        .add_systems(Update, set_next_state.run_if(in_state(MyState::InitialState)))
        .add_systems(Update, next_another_state.run_if(in_state(MyState::AnotherState)))
        .add_systems(Update, in_another_state.run_if(in_state(InAnotherState)))
        .init_state::<MyState>()
        .add_computed_state::<InAnotherState>();
});

