use std::{
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Duration,
};

use camino::{Utf8Path, Utf8PathBuf};
use dexterous_developer_types::{HotReloadMessage, Target};
use futures_util::StreamExt;
use tokio::{io::AsyncWriteExt, time::sleep};
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};
use url::Url;

use crate::{dylib_runner_message::DylibRunnerMessage, error::DylibRunnerError};

pub fn connect_to_server(
    working_directory: &Utf8Path,
    library_path: &Utf8Path,
    server: url::Url,
    tx: async_channel::Sender<DylibRunnerMessage>,
) -> Result<JoinHandle<Result<(), DylibRunnerError>>, DylibRunnerError> {
    let current_target = Target::current().ok_or(DylibRunnerError::NoCurrentTarget)?;

    let address = server.join("target/")?;
    info!("Setting Up Route {address}");
    let mut address = address.join(current_target.as_str())?;
    let initial_scheme = address.scheme();
    let new_scheme = match initial_scheme {
        "http" => "ws",
        "https" => "wss",
        "ws" => "ws",
        "wss" => "wss",
        scheme => {
            return Err(DylibRunnerError::InvalidScheme(
                server.clone(),
                scheme.to_string(),
            ))
        }
    };

    address
        .set_scheme(new_scheme)
        .map_err(|_e| DylibRunnerError::InvalidScheme(server.clone(), "Unknown".to_string()))?;

    let server = server.clone();
    let address = address.clone();
    let library_path = library_path.to_owned();
    let working_directory = working_directory.to_owned();
    let target = current_target;

    Ok(std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let result = remote_connection(
                    address,
                    server,
                    target,
                    tx.clone(),
                    library_path,
                    working_directory,
                )
                .await;
                let _ = tx.send(DylibRunnerMessage::ConnectionClosed).await;
                result
            })
    }))
}

pub(crate) async fn remote_connection(
    address: Url,
    server: Url,
    target: Target,
    tx: async_channel::Sender<DylibRunnerMessage>,
    library_path: Utf8PathBuf,
    working_directory: Utf8PathBuf,
) -> Result<(), DylibRunnerError> {
    info!("Connecting To {address}");

    let (ws_stream, _) = connect_async(address.to_string()).await?;

    let (_, mut read) = ws_stream.split();

    let (download_tx, mut download_rx) =
        tokio::sync::mpsc::unbounded_channel::<(String, Utf8PathBuf, bool)>();

    let mut last_started_id = 0;
    let mut last_completed_id = 0;
    let mut last_triggered_id = 0;
    let mut root_lib_path: Option<Utf8PathBuf> = None;
    let pending_downloads = Arc::new(AtomicU32::new(0));

    loop {
        tokio::select! {
            Some((name, local_path, is_asset)) = download_rx.recv() => {
                if is_asset {
                    info!("downloaded asset {name}");
                    let _ = tx.send(DylibRunnerMessage::AssetUpdated { local_path, name }).await;
                } else if pending_downloads.load(Ordering::SeqCst) == 0 {
                    info!("all downloads completed");
                    if last_completed_id == last_started_id && last_completed_id != last_triggered_id {
                        if let Some(local_path) = root_lib_path.as_ref().cloned() {
                            if local_path.exists() || ({
                                info!("waiting for library to be created");
                                sleep(Duration::from_millis(100)).await;
                                local_path.exists()
                            }){
                                info!("local root lib exists - triggering a reload");
                                last_triggered_id = last_completed_id;
                                let e = tx.send(DylibRunnerMessage::LoadRootLib { build_id: last_triggered_id, local_path }).await;
                                info!("Sent Reload Trigger: {e:?}");
                            } else {
                                info!("local root doesn't exist yet - did download actually complete?");
                            }
                        } else {
                            info!("no local root path exists - not triggering a reload");
                        }
                    } else {
                        info!("last completed is {last_completed_id}, started is {last_started_id} and triggered is {last_triggered_id} - not triggering a reload");
                    }
                }
            }
            Some(msg) = read.next() => {
                let msg = msg?;

                match msg {
                    tokio_tungstenite::tungstenite::Message::Binary(binary) => {
                        let msg: HotReloadMessage = rmp_serde::from_slice(&binary)?;
                        info!("Received Hot Reload Message: {msg:?}");
                        match msg {
                            HotReloadMessage::InitialState {
                                root_lib: initial_root_lib,
                                libraries,
                                assets,
                                most_recent_started_build,
                                most_recent_completed_build,
                                ..
                            } => {
                                info!(r#"Got Initial State:
                                root library: {initial_root_lib:?}
                                most recent started build: {most_recent_started_build}
                                most_recent_completed_build: {most_recent_completed_build}"#);
                                root_lib_path = initial_root_lib.as_ref().map(|path| library_path.join(path));
                                for (path, hash) in libraries {
                                    download_file(&server, target, &library_path, path, hash, pending_downloads.clone(), download_tx.clone(), false);
                                }
                                for (path, hash) in assets {
                                    download_file(&server, target, &working_directory, path, hash, pending_downloads.clone(), download_tx.clone(), true);
                                }
                                last_started_id = most_recent_started_build;
                                last_completed_id = most_recent_completed_build;

                            },
                            HotReloadMessage::RootLibPath(path) => {
                                let local_path = library_path.join(&path);
                                root_lib_path = Some(local_path.clone());
                                info!("root library: {root_lib_path:?}");
                                if pending_downloads.load(Ordering::SeqCst) == 0 {
                                    info!("no remaining downloads");
                                    if last_completed_id == last_started_id && last_completed_id != last_triggered_id {
                                        info!("triggering a reload");
                                        last_triggered_id = last_completed_id;
                                        let _ = tx.send(DylibRunnerMessage::LoadRootLib { build_id: last_triggered_id, local_path }).await;                                    } else {
                                        info!("last completed is {last_completed_id}, started is {last_started_id} and triggered is {last_triggered_id} - not triggering a reload");
                                    }
                                }

                            },
                            HotReloadMessage::UpdatedLibs(path, hash, _) => {
                                download_file(&server,target,  &library_path, Utf8PathBuf::from(path), hash, pending_downloads.clone(), download_tx.clone(), false);
                            },
                            HotReloadMessage::UpdatedAssets(path, hash) => {
                                download_file(&server, target, &working_directory, path, hash, pending_downloads.clone(), download_tx.clone(), true);
                            },
                            HotReloadMessage::BuildStarted(id) => {
                                if id > last_started_id {
                                    info!("build started: {id:?}");
                                    last_started_id = id;
                                }
                            },
                            HotReloadMessage::BuildCompleted(id) => {
                                info!("build completed: {id:?}");
                                if id > last_completed_id {
                                    last_completed_id = id;
                                }
                            },
                            _ => {}
                        }
                    }
                    _ => {
                        warn!("Got Non-Binary WS Message");
                        return Ok(());
                    }
                }
            }
            else => {
                warn!("Download or Reception Failed");
                return Ok(());
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn download_file(
    server: &url::Url,
    target: Target,
    base_path: &Utf8Path,
    remote_path: Utf8PathBuf,
    hash: [u8; 32],
    pending: Arc<AtomicU32>,
    tx: tokio::sync::mpsc::UnboundedSender<(String, Utf8PathBuf, bool)>,
    is_asset: bool,
) {
    info!("Starting Download of {remote_path}");
    pending.fetch_add(1, Ordering::SeqCst);
    let server = server.clone();
    let base_path = base_path.to_owned();
    tokio::spawn(async move {
        let result =
            execute_download(server.clone(), target, base_path, remote_path.clone(), hash).await;
        pending.fetch_sub(1, Ordering::SeqCst);
        match result {
            Ok(path) => {
                let name = remote_path.to_string();
                let mut wait = 0;
                while matches!(tokio::fs::try_exists(&path).await, Err(_) | Ok(false)) && wait < 3 {
                    info!("Waiting for file to exist");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    wait += 1;
                }
                if matches!(tokio::fs::try_exists(&path).await, Err(_) | Ok(false)) {
                    error!("Failed to create file {path}");
                } else {
                    let _ = tx.send((name, path, is_asset));
                }
            }
            Err(e) => {
                error!("Failed To Download File {e:?}");
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn execute_download(
    server: url::Url,
    target: Target,
    base_path: Utf8PathBuf,
    remote_path: Utf8PathBuf,
    hash: [u8; 32],
) -> Result<Utf8PathBuf, DylibRunnerError> {
    let local_path = base_path.join(&remote_path);

    if local_path.exists() {
        let file = tokio::fs::read(&local_path).await?;
        let existing_hash = blake3::hash(&file);
        if hash == *existing_hash.as_bytes() {
            return Ok(local_path);
        }
    }

    let address = server
        .join("files/")?
        .join(&format!("{target}/"))?
        .join(remote_path.as_str())?;
    info!("downloading {remote_path} from {address:?}");
    let req = reqwest::get(address).await?.error_for_status()?;

    let dir = local_path
        .parent()
        .ok_or_else(|| DylibRunnerError::NoAssedDirectory(local_path.clone()))?;
    if !tokio::fs::try_exists(dir).await.unwrap_or(false) {
        tokio::fs::create_dir_all(dir).await?;
    }

    let bytes = req.bytes().await?;

    let mut file = tokio::fs::File::create(&local_path).await?;

    file.write_all(&bytes).await?;
    info!("downloaded {remote_path}");

    Ok(local_path)
}
