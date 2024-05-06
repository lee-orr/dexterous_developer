#![allow(non_snake_case)]

use std::{
    ffi::c_void,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use camino::{Utf8Path, Utf8PathBuf};
use crossbeam::atomic::AtomicCell;

use dexterous_developer_internal::{hot::HotReloadInfoBuilder, CallResponse, UpdatedAsset};
use dexterous_developer_types::{cargo_path_utils::dylib_path, HotReloadMessage, Target};
use futures_util::StreamExt;
use once_cell::sync::OnceCell;
use safer_ffi::{ffi_export, prelude::c_slice};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::connect_async;
use tracing::{error, info, warn};
use url::Url;

use dexterous_developer_internal::library_holder::LibraryHolder;

pub fn run_reloadable_app(
    working_directory: &Utf8Path,
    library_path: &Utf8Path,
    server: url::Url,
) -> Result<(), DylibRunnerError> {
    if !library_path.exists() {
        return Err(DylibRunnerError::LibraryDirectoryDoesntExist(
            library_path.to_owned(),
        ));
    }
    if !working_directory.exists() {
        return Err(DylibRunnerError::WorkingDirectoryDoesntExist(
            working_directory.to_owned(),
        ));
    }

    let dylib_paths = dylib_path();
    if !dylib_paths.contains(&library_path.to_owned()) {
        return Err(DylibRunnerError::DylibPathsMissingLibraries);
    }

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

    let (tx, rx) = async_channel::unbounded::<DylibRunnerMessage>();

    let handle = {
        let server = server.clone();
        let address = address.clone();
        let library_path = library_path.to_owned();
        let working_directory = working_directory.to_owned();
        let target = current_target;

        std::thread::spawn(move || {
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
        })
    };

    let (initial, id) = {
        info!("Getting Initial Root");
        let mut library = None;
        let mut id = None;
        loop {
            if library.is_some() || id.is_some() {
                warn!("We have a root set already...");
            }
            let initial = rx.recv_blocking()?;
            warn!("Got Message While Looking For Root");
            match initial {
                DylibRunnerMessage::ConnectionClosed => {
                    let _ = handle.join().map_err(DylibRunnerError::JoinHandleFailed)?;
                    return Ok(());
                }
                DylibRunnerMessage::LoadRootLib {
                    build_id,
                    local_path,
                } => {
                    info!("Loading Initial Root");
                    library = Some(LibraryHolder::new(&local_path, false)?);
                    id = Some(build_id);
                    break;
                }
                DylibRunnerMessage::AssetUpdated { .. } => {
                    continue;
                }
            }
        }
        info!("Initial Root ID: {id:?}");
        (
            library.ok_or(DylibRunnerError::NoInitialLibrary)?,
            id.ok_or(DylibRunnerError::NoInitialLibrary)?,
        )
    };

    let initial = Arc::new(initial);

    NEXT_UPDATE_VERSION.store(id, std::sync::atomic::Ordering::SeqCst);
    NEXT_LIBRARY.store(Some(initial.clone()));
    ORIGINAL_LIBRARY
        .set(initial.clone())
        .map_err(|_| DylibRunnerError::OnceCellError)?;

    let _handle = std::thread::spawn(|| update_loop(rx, handle));

    info!("Setting Info");

    let info = HotReloadInfoBuilder {
        internal_last_update_version: last_update_version,
        internal_update_ready: update_ready,
        internal_set_update_callback: update_callback,
        internal_update: update,
        internal_set_asset_update_callback: update_asset_callback,
        internal_validate_setup: validate_setup,
    }
    .build();

    initial.varied_call("dexterous_developer_internal_set_hot_reload_info", info)?;
    initial.varied_call(
        "load_internal_library",
        safer_ffi::String::from(initial.path().as_str()),
    )?;

    info!("Calling Internal Main");
    initial.call("dexterous_developer_internal_main", &mut ())?;

    info!("Done.");

    Ok(())
}

async fn remote_connection(
    address: Url,
    server: Url,
    target: Target,
    tx: async_channel::Sender<DylibRunnerMessage>,
    library_path: Utf8PathBuf,
    working_directory: Utf8PathBuf,
) -> Result<(), DylibRunnerError> {
    info!("Connecting To {address}");

    let (ws_stream, _) = connect_async(address).await?;

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
                                sleep(Duration::from_millis(100));
                                local_path.exists()
                            }){
                                info!("local root lib exists -triggering a reload");
                                last_triggered_id = last_completed_id;
                                let e = tx.send(DylibRunnerMessage::LoadRootLib { build_id: last_triggered_id, local_path }).await;
                                error!("Sent Reload Trigger: {e:?}");
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
                        return Ok(());
                    }
                }
            }
            else => {
                return Ok(());
            }
        };
    }
}

#[allow(clippy::too_many_arguments)]
fn download_file(
    server: &url::Url,
    target: Target,
    base_path: &Utf8Path,
    remote_path: Utf8PathBuf,
    _hash: [u8; 32],
    pending: Arc<AtomicU32>,
    tx: tokio::sync::mpsc::UnboundedSender<(String, Utf8PathBuf, bool)>,
    is_asset: bool,
) {
    info!("Starting Download of {remote_path}");
    pending.fetch_add(1, Ordering::SeqCst);
    let server = server.clone();
    let base_path = base_path.to_owned();
    tokio::spawn(async move {
        let result = execute_download(server.clone(), target, base_path, remote_path.clone()).await;
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
) -> Result<Utf8PathBuf, DylibRunnerError> {
    let local_path = base_path.join(&remote_path);

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

fn update_loop(
    rx: async_channel::Receiver<DylibRunnerMessage>,
    handle: std::thread::JoinHandle<Result<(), DylibRunnerError>>,
) -> Result<(), DylibRunnerError> {
    info!("Starting Secondary Update Loop");
    loop {
        let message = rx.recv_blocking()?;
        match message {
            DylibRunnerMessage::ConnectionClosed => {
                let _ = handle.join().map_err(DylibRunnerError::JoinHandleFailed)?;
                eprintln!("Connection Closed");
                return Ok(());
            }
            DylibRunnerMessage::LoadRootLib {
                build_id,
                local_path,
            } => {
                let library = LibraryHolder::new(&local_path, false)?;
                NEXT_UPDATE_VERSION.store(build_id, std::sync::atomic::Ordering::SeqCst);
                NEXT_LIBRARY.store(Some(Arc::new(library)));
                unsafe {
                    if let Some(Some(callback)) = UPDATED_CALLBACK.as_ptr().as_ref() {
                        callback(build_id);
                    }
                }
            }
            DylibRunnerMessage::AssetUpdated { local_path, name } => unsafe {
                if let Some(Some(callback)) = UPDATED_ASSET_CALLBACK.as_ptr().as_ref() {
                    let inner_local_path = c_slice::Box::from(
                        local_path
                            .to_string()
                            .as_bytes()
                            .iter()
                            .copied()
                            .collect::<Box<[u8]>>(),
                    );
                    let inner_name =
                        c_slice::Box::from(name.as_bytes().iter().copied().collect::<Box<[u8]>>());
                    callback(UpdatedAsset {
                        inner_name,
                        inner_local_path,
                    });
                }
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum DylibRunnerMessage {
    ConnectionClosed,
    LoadRootLib {
        build_id: u32,
        local_path: Utf8PathBuf,
    },
    AssetUpdated {
        local_path: Utf8PathBuf,
        name: String,
    },
}

#[derive(Error, Debug)]
pub enum DylibRunnerError {
    #[error("Dylib Runner IO Error {0}")]
    IoError(#[from] std::io::Error),
    #[error("Dynamic Library Paths don't include current library path")]
    DylibPathsMissingLibraries,
    #[error("Couldn't determine current Target")]
    NoCurrentTarget,
    #[error("Couldn't parse URL {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("Couldn't set websocket scheme for {0:?} - {1} is an invalid scheme")]
    InvalidScheme(url::Url, String),
    #[error("Working Directory does not exist - {0:?}")]
    WorkingDirectoryDoesntExist(Utf8PathBuf),
    #[error("Library Directory does not exist - {0:?}")]
    LibraryDirectoryDoesntExist(Utf8PathBuf),
    #[error("WebSocket Error {0}")]
    WebSocketError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("RMP Parse Error {0}")]
    RmpParseError(#[from] rmp_serde::decode::Error),
    #[error("Async Channel Failed {0}")]
    AsyncChannelError(#[from] async_channel::RecvError),
    #[error("Join Handle Failed")]
    JoinHandleFailed(std::boxed::Box<(dyn std::any::Any + std::marker::Send + 'static)>),
    #[error("Library Holder Error {0}")]
    LibraryError(#[from] dexterous_developer_internal::library_holder::LibraryError),
    #[error("Couldn't Open Initial Library")]
    NoInitialLibrary,
    #[error("Original Library Already Set")]
    OnceCellError,
    #[error("Download Failed: {0}")]
    DownloadError(#[from] reqwest::Error),
    #[error("Couldn'y Determine Downloaded Asset Directory: {0}")]
    NoAssedDirectory(Utf8PathBuf),
}

pub static LAST_UPDATE_VERSION: AtomicU32 = AtomicU32::new(0);
pub static NEXT_UPDATE_VERSION: AtomicU32 = AtomicU32::new(0);
pub static ORIGINAL_LIBRARY: OnceCell<Arc<LibraryHolder>> = OnceCell::new();
pub static LAST_LIBRARY: AtomicCell<Option<Arc<LibraryHolder>>> = AtomicCell::new(None);
pub static CURRENT_LIBRARY: AtomicCell<Option<Arc<LibraryHolder>>> = AtomicCell::new(None);
pub static NEXT_LIBRARY: AtomicCell<Option<Arc<LibraryHolder>>> = AtomicCell::new(None);
pub static UPDATED_CALLBACK: AtomicCell<Option<extern "C" fn(u32) -> ()>> = AtomicCell::new(None);
pub static UPDATED_ASSET_CALLBACK: AtomicCell<Option<extern "C" fn(UpdatedAsset) -> ()>> =
    AtomicCell::new(None);

#[ffi_export]
extern "C" fn validate_setup(value: u32) -> u32 {
    println!("Validating Setup - Received {value}");
    value
}

#[ffi_export]
extern "C" fn last_update_version() -> u32 {
    println!("Checking Last Update Version");
    LAST_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst)
}

#[ffi_export]
extern "C" fn update_ready() -> bool {
    println!("Running the readiness check");
    let last = LAST_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    let next = NEXT_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    info!("Checking Readiness: {last} {next}");
    next > last
}

#[ffi_export]
extern "C" fn update() -> bool {
    println!("Updating");
    let next = NEXT_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    let old = LAST_UPDATE_VERSION.swap(next, std::sync::atomic::Ordering::SeqCst);

    if old < next {
        println!("Updated Version");
        LAST_LIBRARY.store(CURRENT_LIBRARY.swap(NEXT_LIBRARY.take()));
        unsafe {
            if let Some(Some(library)) = CURRENT_LIBRARY.as_ptr().as_ref() {
                let path = library.path();
                if let Some(library) = ORIGINAL_LIBRARY.get() {
                    if let Err(e) = library.varied_call(
                        "load_internal_library",
                        safer_ffi::String::from(path.as_str()),
                    ) {
                        eprintln!("Failed to load library: {e}");
                        return false;  
                    }
                }
            }
        }
        true
    } else {
        println!("Didn't update version");
        false
    }
}

#[ffi_export]
extern "C" fn update_callback(callback: extern "C" fn(u32) -> ()) {
    println!("Setting Update Callback");
    UPDATED_CALLBACK.store(Some(callback))
}

#[ffi_export]
extern "C" fn update_asset_callback(callback: extern "C" fn(UpdatedAsset) -> ()) {
    println!("Setting Asset Callback");
    UPDATED_ASSET_CALLBACK.store(Some(callback))
}
