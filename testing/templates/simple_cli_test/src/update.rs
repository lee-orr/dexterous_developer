use bevy::prelude::*;
use dexterous_developer::*;

fn update() {
    println!("Ran Update");
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

pub struct MyPlugin;

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup)
            .add_systems(Update, update);
    }
}
