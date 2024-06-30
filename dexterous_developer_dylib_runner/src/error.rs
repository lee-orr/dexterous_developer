use camino::Utf8PathBuf;
use thiserror::Error;

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
    LibraryError(#[from] dexterous_developer_instance::library_holder::LibraryError),
    #[error("Couldn't Open Initial Library")]
    NoInitialLibrary,
    #[error("Original Library Already Set")]
    OnceCellError,
    #[error("Download Failed: {0}")]
    DownloadError(#[from] reqwest::Error),
    #[error("Couldn'y Determine Downloaded Asset Directory: {0}")]
    NoAssedDirectory(Utf8PathBuf),
}
