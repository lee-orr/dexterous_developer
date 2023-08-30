use std::{
    collections::BTreeSet,
    env::consts::{ARCH, OS},
    path::PathBuf,
    process::Command,
};

use anyhow::bail;
use dexterous_developer_internal::internal_shared::cargo_path_utils::{
    dylib_path, dylib_path_envvar,
};
use futures_util::StreamExt;
use url::Url;

pub async fn connect_to_remote(remote: Url, reload_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let reload_dir = if let Some(reload_dir) = reload_dir {
        reload_dir
    } else {
        std::env::current_dir()?
    };

    if !reload_dir.exists() {
        println!("Creating Reload Directory - {reload_dir:?}");
        std::fs::create_dir_all(reload_dir.as_path())?;
    }

    let lib_dir = reload_dir.join("libs");
    let asset_dir = reload_dir.join("assets");

    if !lib_dir.exists() {
        println!("Creating Library Directory - {lib_dir:?}");
        std::fs::create_dir_all(lib_dir.as_path())?;
    }

    if !asset_dir.exists() {
        println!("Creating Asset Directory - {asset_dir:?}");
        std::fs::create_dir_all(asset_dir.as_path())?;
    }

    let env_paths = dylib_path();

    if !env_paths.contains(&lib_dir) {
        let mut env_paths = env_paths
            .into_iter()
            .filter(|v| !v.as_os_str().is_empty())
            .collect::<BTreeSet<_>>();
        env_paths.insert(lib_dir);

        let os_paths = std::env::join_paths(env_paths)?;

        let current = std::env::current_exe()?;
        let result = Command::new(current)
            .env(dylib_path_envvar(), os_paths.as_os_str())
            .status()
            .expect("Couldn't execute executable");
        std::process::exit(result.code().unwrap_or_default());
    } else {
        let targets = get_valid_targets(&remote).await?;
        println!("TARGETS AVAILABLE {targets:?}");
        let target = targets.first().ok_or(anyhow::Error::msg(format!(
            "No valid targets at {remote:?}"
        )))?;
        println!("Connecting to {remote:?} for target{target}");
    }
    Ok(())
}

async fn get_valid_targets(url: &url::Url) -> anyhow::Result<Vec<String>> {
    let result = reqwest::get(url.join("targets")?).await?;
    let json = result.json::<Vec<String>>().await?;
    let filtered: Vec<String> = json
        .into_iter()
        .filter(|v| v.contains(ARCH) && v.contains(OS))
        .collect();
    if filtered.is_empty() {
        bail!("No valid targets available at {url:?}");
    }
    Ok(filtered)
}

async fn connect_to_build(url: &url::Url, target: String) -> anyhow::Result<()> {
    let url = url.join("connect")?.join(&target)?;
    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    let (_write, _read) = ws_stream.split();

    todo!()
}
