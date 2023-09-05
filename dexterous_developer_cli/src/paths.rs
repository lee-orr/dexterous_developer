use std::path::PathBuf;

use anyhow::Context;
use directories::ProjectDirs;

pub fn get_paths() -> anyhow::Result<CliPaths> {
    let dirs = ProjectDirs::from("git", "dexterous_developer", "dexterous_developer_cli")
        .context("Couldn't get application directories")?;

    let data = dirs.data_dir().to_path_buf();

    if !data.exists() {
        println!("Setting up data directory at {data:?}");
        std::fs::create_dir_all(data.as_path())?;
    }

    Ok(CliPaths {
        cross_config: data.join("cross_config.toml"),
        data,
    })
}

pub struct CliPaths {
    pub data: PathBuf,
    pub cross_config: PathBuf,
}
