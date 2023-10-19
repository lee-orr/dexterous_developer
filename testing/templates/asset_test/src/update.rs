use std::{env, path::PathBuf};

use bevy::{
    asset::UpdateAssets,
    prelude::{AssetServer, Assets, Commands, Res, Startup, Update},
};
use dexterous_developer::*;

use crate::{Text, TextAsset};

fn update(text: Res<Text>, texts: Res<Assets<TextAsset>>) {
    for (id, text) in texts.iter() {
        println!("Got id: {id:?} and text {text:?}");
    }
    let Some(text) = texts.get(&text.0) else {
        eprintln!("No Asset");
        return;
    };
    println!("Asset: {}", text.value);
}

fn startup(asset_server: Res<AssetServer>, mut commands: Commands) {
    println!("Press Enter to Progress, or type 'exit' to exit");
    let base_path = get_base_path();
    println!("Using assets at: {base_path:?}/assets");
    let text = asset_server.load("nesting/another_placeholder.txt");
    commands.insert_resource(Text(text))
}

fn asset_updates() {
    println!("Running asset updates...");
}

#[dexterous_developer_setup]
pub fn reloadable(app: &mut ReloadableAppContents) {
    app.add_systems(Startup, startup)
        .add_systems(Update, update)
        .add_systems(UpdateAssets, asset_updates);
}

pub(crate) fn get_base_path() -> PathBuf {
    if let Ok(manifest_dir) = env::var("BEVY_ASSET_ROOT") {
        PathBuf::from(manifest_dir)
    } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
    } else {
        env::current_exe()
            .map(|path| {
                path.parent()
                    .map(|exe_parent_path| exe_parent_path.to_owned())
                    .unwrap()
            })
            .unwrap()
    }
}
