use std::{
    collections::BTreeSet,
    env::consts::{ARCH, OS},
    io::copy,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context};
use dexterous_developer_internal::{
    internal_shared::cargo_path_utils::{dylib_path, dylib_path_envvar},
    HotReloadMessage,
};
use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub async fn connect_to_remote(remote: Url, reload_dir_rel: Option<PathBuf>) -> anyhow::Result<()> {
    let mut reload_dir = std::env::current_dir()?;
    if let Some(reload_dir_rel) = reload_dir_rel {
        reload_dir.push(reload_dir_rel);
    }

    if !reload_dir.exists() {
        println!("Creating Reload Directory - {reload_dir:?}");
        std::fs::create_dir_all(reload_dir.as_path())?;
    }

    let reload_dir = reload_dir.canonicalize()?;

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
            .current_dir(reload_dir.as_path())
            .env(dylib_path_envvar(), os_paths.as_os_str())
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
        {
            let target = target.clone();
            let remote = remote.clone();
            let lib_dir = lib_dir.clone();
            tokio::spawn(async move {
                connect_to_build(
                    &remote,
                    target.as_str(),
                    lib_dir.as_path(),
                    lib_name_tx,
                    paths_ready_tx,
                )
                .await
                .expect("Couldn't connect to build");
            });
        }
        let path = lib_name_rx
            .recv()
            .await
            .ok_or(anyhow::Error::msg("Couldn't get root lib path"))?;
        println!("Starting App - got {path:?}");
        if path.exists() {
            println!("{path:?} exists");
        } else {
            bail!("{path:?} doesn't exist");
        }
        paths_ready_rx
            .recv()
            .await
            .ok_or(anyhow::Error::msg("Couldn't download all paths"))?;
        dexterous_developer_internal::run_served_file(path).await?;
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

async fn connect_to_build(
    root_url: &url::Url,
    target: &str,
    target_folder: &Path,
    lib_path_tx: mpsc::Sender<PathBuf>,
    paths_ready: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    let mut url = root_url.join(&format!("connect/{target}"))?;
    let _ = url.set_scheme("ws");

    println!("Connecting to {url}");

    let (ws_stream, _) = tokio_tungstenite::connect_async(url).await?;

    let (_write, read) = ws_stream.split();

    let lib_path_ref = &lib_path_tx;
    let paths_ready = &paths_ready;

    read.for_each(|msg| async move {
        println!("Got Message {msg:?}");
        if let Ok(Message::Text(msg)) = msg {
            let Ok(msg) = serde_json::from_str::<HotReloadMessage>(&msg) else {
                eprintln!("Couldn't parse message");
                return;
            };
            match msg {
                HotReloadMessage::RootLibPath(root_path) => {
                    println!("Downloading root lib to {root_path}");
                    if download_to_folder(root_url, target, &root_path, target_folder)
                        .await
                        .is_err()
                    {
                        eprintln!("Couldn't download {root_path:?}");
                    } else {
                        let root_path = target_folder.join(root_path);

                        println!("Got root lib - at path {root_path:?}");
                        let _ = lib_path_ref.send(root_path).await;
                    }
                }
                HotReloadMessage::UpdatedPaths(updated_paths) => {
                    for file in updated_paths {
                        println!("Downloading {file}");
                        if download_to_folder(root_url, target, &file, target_folder)
                            .await
                            .is_err()
                        {
                            eprintln!("Couldn't download {file:?}");
                        } else {
                            println!("Downloaded {file:?}");
                        }
                    }
                    let _ = paths_ready.send(()).await;
                }
            }
        }
    })
    .await;
    Ok(())
}

async fn download_to_folder(
    url: &url::Url,
    target: &str,
    file: &str,
    target_folder: &Path,
) -> anyhow::Result<()> {
    let path = format!("{url}libs/{target}/{file}");
    println!("Downloading {path}");
    let response = reqwest::get(path).await?;
    if !response.status().is_success() {
        bail!("Download failed - {response:?}");
    }
    let content = response.bytes().await?;
    let target = target_folder.join(file);
    println!("Downloading to {target:?}");
    tokio::fs::write(target, content)
        .await
        .context("Write to {target:?} failed")?;
    Ok(())
}
