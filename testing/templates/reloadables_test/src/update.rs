use bevy::prelude::{Component, Resource, Startup, Update};
use dexterous_developer::*;
use serde::{Deserialize, Serialize};

fn update() {
    println!("Ran Update");
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

#[dexterous_developer_setup]
pub fn reloadable(app: &mut ReloadableAppContents) {
    app.add_systems(Startup, startup)
        .add_systems(Update, update);
}

#[derive(Component, Serialize, Deserialize, Default)]
struct TextComponent(String);

impl ReplacableComponent for TextComponent {
    fn get_type_name() -> &'static str {
        "text_component"
    }
}

#[derive(Resource, Serialize, Deserialize, Default)]
struct TextResource(String);

impl ReplacableResource for TextResource {
    fn get_type_name() -> &'static str {
        "text_resource"
    }
}
