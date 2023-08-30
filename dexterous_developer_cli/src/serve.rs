use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::Request,
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
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

pub async fn run_server(
    port: u16,
    package: Option<String>,
    features: Vec<String>,
    prefer_mold: bool,
) -> Result<()> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/libs/:target/:file", get(target_file_loader))
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
    println!("Serving on {port}");
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
        let (updates_rx, lib_path, lib_dir) = state.get_target_connection(&target).await;

        if let Ok(dir_content) = lib_dir.read_dir() {
            let paths = dir_content
                .filter_map(|v| v.ok())
                .map(|v| (v.file_name().to_string_lossy().to_string(), v.path()))
                .filter_map(|(v, f)| std::fs::read(f).ok().map(|f| (v, f)))
                .map(|(name, f)| {
                    let hash = blake3::hash(&f);

                    (name, hash.as_bytes().to_owned())
                })
                .collect::<Vec<_>>();
            let Ok(content) = serde_json::to_string(&HotReloadMessage::UpdatedPaths(paths)) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            let _ = sender.send(Message::Text(content)).await;
        }

        {
            let Ok(content) = serde_json::to_string(&HotReloadMessage::RootLibPath(lib_path))
            else {
                eprintln!("Couldn't serialize current path - closing connection");
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

async fn target_file_loader(
    Path((target, file)): Path<(String, String)>,
    state: State<ServerState>,
    request: Request<Body>,
) -> Result<Response> {
    println!("Requested file {file} from {target}");
    let dir = {
        let map = state.map.read().await;
        let target = map
            .get(target.as_str())
            .ok_or(anyhow::Error::msg("Couldn't get target"))?;
        target.lib_dir.clone()
    };
    let file = dir.join(file);
    println!("File path {file:?}");
    let serve = ServeFile::new(file);
    let result = serve.oneshot(request).await?;
    println!("Result has status {:?}", result.status());
    Ok(result.into_response())
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
    ) -> (Receiver<HotReloadMessage>, String, PathBuf) {
        {
            if let Some(map) = self.map.read().await.get(target) {
                return (
                    map.sender.subscribe(),
                    map.lib_path.clone(),
                    map.lib_dir.clone(),
                );
            }
        }
        let (sender, receiver) = broadcast::channel(100);

        let mut map = self.map.write().await;
        if let Some(map) = map.get(target) {
            return (
                map.sender.subscribe(),
                map.lib_path.clone(),
                map.lib_dir.clone(),
            );
        }

        let options = HotReloadOptions {
            features: self.features.clone(),
            package: self.package.clone(),
            prefer_mold: self.prefer_mold,
            build_target: Some(target.to_string()),
            ..Default::default()
        };
        let (lib_path, lib_dir) =
            watch_reloadable(options, sender.clone()).expect("Couldn't setup reloadable");

        let lib_path = lib_path
            .file_name()
            .unwrap_or(lib_path.as_os_str())
            .to_string_lossy()
            .to_string();

        let target_watch_info = TargetWatchInfo {
            sender: sender.clone(),
            lib_path: lib_path.clone(),
            lib_dir: lib_dir.clone(),
        };
        map.insert(target.to_string(), target_watch_info);
        (receiver, lib_path, lib_dir)
    }
}

#[derive(Clone)]
struct TargetWatchInfo {
    sender: broadcast::Sender<HotReloadMessage>,
    lib_path: String,
    lib_dir: PathBuf,
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
