use camino::{Utf8Path, Utf8PathBuf};
use dexterous_developer_types::{cargo_path_utils::dylib_path, HotReloadMessage, Target};
use futures_util::StreamExt;
use thiserror::Error;
use tokio_tungstenite::connect_async;
use tracing::info;

pub async fn run_reloadable_app(
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

    let mut address = server.join("connect")?.join(current_target.as_str())?;
    let initial_scheme = address.scheme();
    let new_scheme = match initial_scheme {
        "http" => "ws",
        "https" => "wss",
        "ws" => "ws",
        "wss" => "wss",
        scheme => return Err(DylibRunnerError::InvalidScheme(server, scheme.to_string())),
    };

    address
        .set_scheme(new_scheme)
        .map_err(|_e| DylibRunnerError::InvalidScheme(server, "Unknown".to_string()))?;

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
    RmpPArseError(#[from] rmp_serde::decode::Error),
}
