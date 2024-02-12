use std::{collections::BTreeSet, path::PathBuf, process::Command};

use anyhow::{bail, Context};
use dexterous_developer_types::cargo_path_utils::{dylib_path, dylib_path_envvar};

pub async fn load_existing_directory(libs: PathBuf) -> anyhow::Result<()> {
    let current_dir = std::env::current_dir().context("Couldn't get current working directory")?;

    let libs = if libs.is_absolute() {
        libs
    } else {
        current_dir.join(libs)
    };

    if !libs.exists() {
        bail!("Library directory doesn't exist");
    }

    let assets = current_dir.join("assets");

    if !assets.exists() {
        bail!("No assets directory within the current directory...");
    }

    let libs = dunce::canonicalize(libs)?;
    let _assets = dunce::canonicalize(assets)?;

    let current_lib = libs
        .read_dir()?
        .find_map(|lib| {
            if let Ok(lib) = lib {
                if lib.file_name().to_string_lossy().ends_with("backup") {
                    return Some(lib);
                }
            }
            None
        })
        .map(|lib| -> anyhow::Result<PathBuf> {
            let current_path = lib.path();
            let new_path = current_path.with_extension("");
            std::fs::copy(current_path, &new_path)?;
            Ok(new_path)
        })
        .context("Couldn't find a root library to start from")??;

    let env_paths = dylib_path();

    if !env_paths.contains(&libs) {
        let mut env_paths = env_paths
            .into_iter()
            .filter(|v| !v.as_os_str().is_empty())
            .collect::<BTreeSet<_>>();

        env_paths.insert(libs.clone());

        let os_paths = std::env::join_paths(env_paths)?;

        println!("Paths: {os_paths:?}");

        let current = std::env::current_exe()?;
        let result = Command::new(current)
            .env(dylib_path_envvar(), os_paths.as_os_str())
            .arg("run-existing")
            .arg(libs.as_os_str())
            .status()
            .expect("Couldn't execute executable");
        std::process::exit(result.code().unwrap_or_default());
    } else {
        println!("Running lib with root lib {current_lib:?}");
        dexterous_developer_internal::run_existing_library(current_lib).await?;
    }

    Ok(())
}
