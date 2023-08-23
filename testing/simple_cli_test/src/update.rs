use bevy::prelude::Update;
use dexterous_developer::*;

fn update() {
    println!("Ran Update");
}

#[dexterous_developer_setup]
pub fn reloadable(app: &mut ReloadableAppContents) {
    app.add_systems(Update, update);
}
