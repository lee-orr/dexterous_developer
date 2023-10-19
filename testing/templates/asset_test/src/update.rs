use std::{env, path::PathBuf};

use bevy::prelude::{AssetServer, Assets, Commands, Res};

use crate::{Text, TextAsset};

pub fn update(text: Res<Text>, texts: Res<Assets<TextAsset>>) {
    for (id, text) in texts.iter() {
        println!("Got id: {id:?} and text {text:?}");
    }
    let Some(text) = texts.get(&text.0) else {
        eprintln!("No Asset");
        return;
    };
    println!("Asset: {}", text.value);
}

pub fn startup(asset_server: Res<AssetServer>, mut commands: Commands) {
    println!("Press Enter to Progress, or type 'exit' to exit");
    let base_path = get_base_path();
    println!("Using assets at: {base_path:?}/assets");
    let text = asset_server.load("nesting/another_placeholder.txt");
    commands.insert_resource(Text(text))
}

pub fn asset_updates() {
    println!("Running asset updates...");
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
