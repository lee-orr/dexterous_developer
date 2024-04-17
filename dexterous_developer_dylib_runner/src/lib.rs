pub mod library_holder;

use camino::{Utf8Path, Utf8PathBuf};
use crossbeam::{channel::Sender, thread};
use dexterous_developer_types::{cargo_path_utils::dylib_path, HotReloadMessage, Target};
use futures_util::StreamExt;
use thiserror::Error;
use tokio_tungstenite::connect_async;
use tracing::info;
use url::Url;

use crate::library_holder::LibraryHolder;

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

    let (tx, rx) = crossbeam::channel::unbounded::<DylibRunnerMessage>();

    let handle = {
        let server = server.clone();
        let address = address.clone();
        let library_path = library_path.clone();
        let working_directory = working_directory.clone();

        std::thread::spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on({
                    let result = remote_connection(address, server, tx.clone());
                    let _ = tx.send(DylibRunnerMessage::ConnectionClosed);
                    result
                })
        })
    };

    loop {
        let initial = rx.recv()?;
        match initial {
            DylibRunnerMessage::ConnectionClosed => {
                let _ = handle
                    .join()
                    .map_err(DylibRunnerError::JoinHandleFailed)?;
                return Ok(())
            }
            DylibRunnerMessage::LoadRootLib {
                build_id,
                local_path,
            } => {
                let library = LibraryHolder::new(&local_path)?;
            },
            DylibRunnerMessage::AssetUpdated { .. } => {
                continue;
            },
        }
    }

    Ok(())
}

async fn remote_connection(
    address: Url,
    server: Url,
    tx: Sender<DylibRunnerMessage>,
) -> Result<(), DylibRunnerError> {
    info!("Connecting To {address}");

    let (ws_stream, _) = connect_async(address).await?;

    let (_, mut read) = ws_stream.split();

    loop {
        let Some(msg) = read.next().await else {
            return Ok(());
        };

        let msg = msg?;

        match msg {
            tokio_tungstenite::tungstenite::Message::Binary(binary) => {
                let msg: HotReloadMessage = rmp_serde::from_slice(&binary)?;
                info!("Received Hot Reload Message: {msg:?}");
            }
            _ => {
                return Ok(());
            }
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
    #[error("Crossbeam Channel Failed {0}")]
    CrosbeamChannelError(#[from] crossbeam::channel::RecvError),
    #[error("Join Handle Failed")]
    JoinHandleFailed(std::boxed::Box<(dyn std::any::Any + std::marker::Send + 'static)>),
    #[error("Library Holder Error {0}")]
    LibraryError(#[from]library_holder::LibraryError)
}
