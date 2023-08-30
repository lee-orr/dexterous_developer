use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dexterous_developer_internal::{watch_reloadable, HotReloadMessage, HotReloadOptions};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    process::Command,
    sync::{
        broadcast::{self, Receiver},
        RwLock,
    },
};
use tower_http::services::ServeDir;

pub async fn run_server(
    port: u16,
    package: Option<String>,
    features: Vec<String>,
    prefer_mold: bool,
) -> Result<()> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/connect/:target", get(websocket_connect));

    let asset_directory = std::env::current_dir()?.join("assets");
    let app = if asset_directory.exists() {
        app.nest_service("/assets", ServeDir::new(asset_directory))
    } else {
        app
    };

    let app = app.with_state(ServerState {
        map: Default::default(),
        package,
        features,
        prefer_mold,
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn list_targets() -> Result<Json<Vec<String>>> {
    let targets = Command::new("rustup")
        .arg("target")
        .arg("list")
        .arg("--installed")
        .output()
        .await?;
    let output = std::str::from_utf8(&targets.stdout)?
        .lines()
        .map(|v| v.to_string())
        .collect();
    Ok(Json(output))
}

async fn websocket_connect(
    target: Path<String>,
    ws: WebSocketUpgrade,
    state: State<ServerState>,
) -> impl IntoResponse {
    let target = target.0.clone();
    ws.on_upgrade(|socket| websocket_connection(socket, target, state))
}

async fn websocket_connection(socket: WebSocket, target: String, state: State<ServerState>) {
    let (mut sender, _) = socket.split();
    let mut updates_rx = {
        let (updates_rx, lib_path, current_files) = state.get_target_connection(&target).await;
        if let Some(lib_path) = lib_path {
            let Ok(content) = serde_json::to_string(&HotReloadMessage::RootLibPath(lib_path))
            else {
                eprintln!("Couldn't serialize current path - closing connection");
                return;
            };
            let _ = sender.send(Message::Text(content)).await;
        }
        if !current_files.is_empty() {
            let Ok(content) = serde_json::to_string(&HotReloadMessage::UpdatedPaths(
                current_files.into_iter().collect(),
            )) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            let _ = sender.send(Message::Text(content)).await;
        }
        updates_rx
    };
    while let Ok(msg) = updates_rx.recv().await {
        let Ok(content) = serde_json::to_string(&msg) else {
            eprintln!("Couldn't serialize current updated paths - closing connection");
            continue;
        };
        let _ = sender.send(Message::Text(content)).await;
    }
}

#[derive(Clone)]
pub struct ServerState {
    map: Arc<RwLock<HashMap<String, TargetWatchInfo>>>,
    package: Option<String>,
    features: Vec<String>,
    prefer_mold: bool,
}

impl ServerState {
    async fn get_target_connection(
        &self,
        target: &str,
    ) -> (
        Receiver<HotReloadMessage>,
        Option<PathBuf>,
        HashSet<PathBuf>,
    ) {
        {
            if let Some(map) = self.map.read().await.get(target) {
                return (
                    map.sender.subscribe(),
                    map.lib_path.read().await.clone(),
                    map.lib_files.read().await.clone(),
                );
            }
        }
        let (sender, receiver) = broadcast::channel(100);
        let lib_path: Arc<RwLock<Option<PathBuf>>> = Default::default();
        let lib_files: Arc<RwLock<HashSet<PathBuf>>> = Default::default();
        let target_watch_info = TargetWatchInfo {
            sender: sender.clone(),
            lib_path: lib_path.clone(),
            lib_files: lib_files.clone(),
        };

        {
            let mut map = self.map.write().await;
            if let Some(map) = map.get(target) {
                return (
                    map.sender.subscribe(),
                    map.lib_path.read().await.clone(),
                    map.lib_files.read().await.clone(),
                );
            }
            map.insert(target.to_string(), target_watch_info);
        }

        let options = HotReloadOptions {
            features: self.features.clone(),
            package: self.package.clone(),
            prefer_mold: self.prefer_mold,
            build_target: Some(target.to_string()),
            ..Default::default()
        };
        {
            let lib_path = lib_path.clone();
            let lib_files = lib_files.clone();
            tokio::spawn(async move {
                let mut rx = sender.subscribe();
                watch_reloadable(options, sender)
                    .await
                    .expect("Couldn't set up watcher");
                while let Ok(recv) = rx.recv().await {
                    match recv {
                        HotReloadMessage::RootLibPath(path) => {
                            let mut writer = lib_path.write().await;
                            let _ = writer.insert(path);
                        }
                        HotReloadMessage::UpdatedPaths(paths) => {
                            let mut current = lib_files.write().await;
                            for path in paths.into_iter() {
                                current.insert(path);
                            }
                        }
                    }
                }
            });
        }
        (receiver, None, Default::default())
    }
}

#[derive(Clone)]
struct TargetWatchInfo {
    sender: broadcast::Sender<HotReloadMessage>,
    lib_path: Arc<RwLock<Option<PathBuf>>>,
    lib_files: Arc<RwLock<HashSet<PathBuf>>>,
}

// Make our own error that wraps `anyhow::Error`.
#[derive(Debug)]
pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

type Result<T> = std::result::Result<T, AppError>;
