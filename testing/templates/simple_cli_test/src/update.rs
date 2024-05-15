use bevy::prelude::{Startup, Update};
use bevy_dexterous_developer::*;

fn update() {
    println!("Ran Update.");
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

reloadable_scope!(reloadable(app) {
    println!("Setting Up Reloadable Scope");
    app.add_systems(Startup, startup)
        .add_systems(Update, update);
    println!("Setup Scope");
});
