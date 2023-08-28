use bevy::prelude::{Resource, States};

#[derive(Resource, Debug, Default)]
pub struct StdInput(pub String);

#[derive(States, PartialEq, Eq, Clone, Copy, Debug, Hash, Default)]
pub enum AppState {
    #[default]
    State,
    AnotherState,
}
