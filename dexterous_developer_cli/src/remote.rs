use std::{
    collections::BTreeSet,
    env::consts::{ARCH, OS},
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context};
use blake3::Hash;
use dexterous_developer_internal::{
    internal_shared::cargo_path_utils::{dylib_path, dylib_path_envvar},
    HotReloadMessage,
};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub async fn connect_to_remote(remote: Url, reload_dir_rel: Option<PathBuf>) -> anyhow::Result<()> {
    let mut reload_dir = PathBuf::from(std::env::current_dir()?.to_string_lossy().to_string());

    println!("Reload Directory: {reload_dir:?}");

    if let Some(reload_dir_rel) = reload_dir_rel {
        reload_dir.push(reload_dir_rel);
    }

    if !reload_dir.exists() {
        println!("Creating Reload Directory - {reload_dir:?}");
        std::fs::create_dir_all(reload_dir.as_path())?;
    } else {
        println!("Reload Directory Exists");
    }

    let lib_dir = reload_dir.join("libs");
    let asset_dir = reload_dir.join("assets");

    if !lib_dir.exists() {
        println!("Creating Library Directory - {lib_dir:?}");
        std::fs::create_dir_all(lib_dir.as_path())?;
    } else {
        println!("Lib Directory Exists");
    }

    if !asset_dir.exists() {
        println!("Creating Asset Directory - {asset_dir:?}");
        std::fs::create_dir_all(asset_dir.as_path())?;
    } else {
        println!("Asset Drectory Exists");
    }

    println!("Canonicalizing paths");

    let reload_dir = dunce::canonicalize(reload_dir)?;
    let lib_dir = dunce::canonicalize(lib_dir)?;
    let asset_dir = dunce::canonicalize(asset_dir)?;

    println!("Gettin Dynamic Library Search Paths");

    let env_paths = dylib_path();

    if !env_paths.contains(&lib_dir) {
        let mut env_paths = env_paths
            .into_iter()
            .filter(|v| !v.as_os_str().is_empty())
            .collect::<BTreeSet<_>>();

        env_paths.insert(lib_dir);

        let os_paths = std::env::join_paths(env_paths)?;

        println!("Paths: {os_paths:?}");

        let current = std::env::current_exe()?;
        let result = Command::new(current)
            .current_dir(reload_dir.as_path())
            .env(dylib_path_envvar(), os_paths.as_os_str())
            .arg("remote")
            .arg("--remote")
            .arg(remote.as_str())
            .status()
            .expect("Couldn't execute executable");
        std::process::exit(result.code().unwrap_or_default());
    } else {
        let targets = get_valid_targets(&remote).await?;
        println!("TARGETS AVAILABLE {targets:?}");
        let target = targets
            .first()
            .ok_or(anyhow::Error::msg(format!("No valid targets at {remote}")))?
            .to_owned();
        println!("Connecting to {remote} for target{target}");
        let (lib_name_tx, mut lib_name_rx) = mpsc::channel(1);
        let (paths_ready_tx, mut paths_ready_rx) = mpsc::channel(1);
        let (assets_ready_tx, mut assets_ready_rx) = mpsc::channel(1);
        {
            let target = target.clone();
            let remote = remote.clone();
            let lib_dir = lib_dir.clone();
            tokio::spawn(async move {
                connect_to_build(
                    &remote,
                    target.as_str(),
                    lib_dir.as_path(),
                    asset_dir.as_path(),
                    lib_name_tx,
                    paths_ready_tx,
                    assets_ready_tx,
                )
                .await
                .expect("Couldn't connect to build");
            });
        }
        let (_lib, path) = lib_name_rx
            .recv()
            .await
            .ok_or(anyhow::Error::msg("Couldn't get root lib path"))?;
        println!("Looking for root lib at {path:?}");
        assets_ready_rx
            .recv()
            .await
            .ok_or(anyhow::Error::msg("Cloudn't load assets"))?;
        loop {
            paths_ready_rx
                .recv()
                .await
                .ok_or(anyhow::Error::msg("Couldn't download all paths"))?;
            if path.exists() {
                println!("{path:?} exists");
                break;
            } else {
                eprintln!("root: {path:?} doesn't exist - waiting for another build...");
            }
        }
        println!("Starting App - got {path:?}");
        dexterous_developer_internal::run_served_file(path).await?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
const CURRENT_OS: &str = "darwin";

#[cfg(not(target_os = "macos"))]
const CURRENT_OS: &str = OS;

async fn get_valid_targets(url: &url::Url) -> anyhow::Result<Vec<String>> {
    let result = reqwest::get(url.join("targets")?).await?;
    let json = result.json::<Vec<String>>().await?;
    let filtered: Vec<String> = json
        .iter()
        .filter(|v| v.contains(ARCH) && v.contains(CURRENT_OS))
        .map(|v| v.to_string())
        .collect();
    if filtered.is_empty() {
        bail!("No valid targets available at {url:?}\nWe're on {ARCH} - {CURRENT_OS}, and got {json:?}");
    }
    Ok(filtered)
}

#[cfg(target_os = "windows")]
const DYNAMIC_LIB_EXTENSION: &str = "dll";
#[cfg(target_os = "linux")]
const DYNAMIC_LIB_EXTENSION: &str = "so";
#[cfg(target_os = "macos")]
const DYNAMIC_LIB_EXTENSION: &str = "dylib";
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
const DYNAMIC_LIB_EXTENSION: &str = "so";

async fn connect_to_build(
    root_url: &url::Url,
    target: &str,
    target_folder: &Path,
    asset_folder: &Path,
    lib_path_tx: mpsc::Sender<(String, PathBuf)>,
    paths_ready: mpsc::Sender<()>,
    assets_ready: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    let mut url = root_url.join(&format!("connect/{target}"))?;
    let _ = url.set_scheme("ws");

    println!("Connecting to {url}");

    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    let (_write, read) = ws_stream.split();

    let lib_path_ref = &lib_path_tx;
    let paths_ready = &paths_ready;
    let assets_ready = &assets_ready;

    read.for_each(|msg| async move {
        println!("Got Message {msg:?}");
        if let Ok(Message::Text(msg)) = msg {
            let Ok(msg) = serde_json::from_str::<HotReloadMessage>(&msg) else {
                eprintln!("Couldn't parse message");
                return;
            };
            match msg {
                HotReloadMessage::RootLibPath(root_lib) => {
                    let root_path = target_folder.join(root_lib.as_str());

                    println!("Got root lib - at path {root_path:?}");
                    let _ = lib_path_ref.send((root_lib, root_path)).await;
                }
                HotReloadMessage::UpdatedLibs(updated_paths) => {
                    for (file, hash) in updated_paths.iter() {
                        if !file.ends_with(DYNAMIC_LIB_EXTENSION) {
                            println!("Ignoring {file} - wrong extension");
                        }
                        println!("Downloading {file} - with {hash:?}");
                        if download_lib(root_url, target, file, hash, target_folder)
                            .await
                            .is_err()
                        {
                            eprintln!("Couldn't download {file:?}");
                        }
                    }
                    println!("Updated Files Downloaded");
                    let _ = paths_ready.send(()).await;
                }
                HotReloadMessage::UpdatedAssets(updated_assets) => {
                    println!("Got Asset List: {updated_assets:?}");
                    for (file, hash) in updated_assets.iter() {
                        println!("Downloading Asset {file} - with {hash:?}");
                        if download_asset(root_url, file, hash, asset_folder)
                            .await
                            .is_err()
                        {
                            eprintln!("Couldn't download {file:?}");
                        }
                    }
                    println!("Updated Assets Downloaded");
                    let _ = assets_ready.send(()).await;
                }
                HotReloadMessage::KeepAlive => println!("Received Keep Alive Message"),
            }
        }
    })
    .await;
    Ok(())
}

async fn download_lib(
    url: &url::Url,
    target: &str,
    file: &str,
    hash: &[u8; 32],
    target_folder: &Path,
) -> anyhow::Result<()> {
    let file_path = target_folder.join(file);
    if file_path.exists() {
        println!("comparing hashes");
        if let Ok(f) = std::fs::read(file_path.as_path()) {
            let hash_2 = blake3::hash(&f);
            let hash = Hash::from_bytes(hash.to_owned());
            if hash_2 == hash {
                println!("{file_path:?} already up to date");
                return Ok(());
            }
        }
    }
    let path = format!("{url}libs/{target}/{file}");
    println!("Downloading {path}");
    let response = reqwest::get(path).await?;
    if !response.status().is_success() {
        bail!("Download failed - {response:?}");
    }
    let content = response.bytes().await?;
    println!("Downloading to {file_path:?}");
    tokio::fs::write(file_path, content)
        .await
        .context("Write to {file_path:?} failed")?;
    println!("Downloaded {file:?}");
    Ok(())
}

async fn download_asset(
    url: &url::Url,
    file: &str,
    hash: &[u8; 32],
    target_folder: &Path,
) -> anyhow::Result<()> {
    let file = file.trim_start_matches('/');
    let file_path = target_folder.join(file);
    if file_path.exists() {
        println!("comparing hashes");
        if let Ok(f) = std::fs::read(file_path.as_path()) {
            let hash_2 = blake3::hash(&f);
            let hash = Hash::from_bytes(hash.to_owned());
            if hash_2 == hash {
                println!("{file_path:?} already up to date");
                return Ok(());
            }
        }
    }
    if let Some(dir) = file_path.parent() {
        if !dir.exists() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let path = format!("{url}assets/{file}");
    println!("Downloading {path}");
    let response = reqwest::get(path).await?;
    if !response.status().is_success() {
        bail!("Download failed - {response:?}");
    }
    let content = response.bytes().await?;
    println!("Downloading to {file_path:?}");
    tokio::fs::write(file_path, content)
        .await
        .context("Write to {target:?} failed")?;
    println!("Downloaded Asset {file:?}");
    Ok(())
}
