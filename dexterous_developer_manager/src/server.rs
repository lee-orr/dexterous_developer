use std::{
    collections::HashMap,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    str::{FromStr, Utf8Error},
    sync::Arc,
    time::Duration,
};

use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Path, Request, State, WebSocketUpgrade,
    },
    http::Response,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use dexterous_developer_builder::command::{
    first_exec, run_broadcaster_with_settings, setup_build_settings,
};
use dexterous_developer_types::{HotReloadMessage, HotReloadOptions, Target, TargetParseError};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::{
    process::Command,
    sync::{
        broadcast::{self, Receiver},
        broadcast, RwLock,
    },
};
use tower::util::ServiceExt;
use tower_http::services::ServeFile;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{Manager, ManagerError};

pub async fn run_server(port: u16, manager: Manager) -> Result<(), ServerError> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/libs/:target/:file", get(target_file_loader))
        .route("/connect/:target", get(websocket_connect));

    let app = app.with_state(ServerState {
        manager: Arc::new(manager),
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}

async fn list_targets(state: State<ServerState>) -> Json<Vec<String>> {
    let output = state
        .manager
        .targets()
        .iter()
        .map(|target| target.to_string())
        .collect();
    Json(output)
}
async fn websocket_connect(
    target: Path<String>,
    ws: WebSocketUpgrade,
    state: State<ServerState>,
) -> impl IntoResponse {
    println!("Connection for target {target:?}");
    let target: Target = match target.0.parse() {
        Ok(target) => target,
        Err(e) => {
            eprintln!("Bad Request - {e:?}");
            return (
                axum::http::StatusCode::BAD_REQUEST,
                "Invalid target {target}",
            )
                .into_response();
        }
    };
    let updates_rx = state.manager.broadcast_target(&target).await;
    match updates_rx {
        Ok(updates_rx) => {
            ws.on_upgrade(move |socket| websocket_connection(socket, target, state, updates_rx))
        }
        Err(e) => (axum::http::StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn websocket_connection(
    socket: WebSocket,
    target: Target,
    state: State<ServerState>,
    update_rx: broadcast::Receiver<HotReloadMessage>,
) {
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
            let Ok(content) = rmp_serde::to_vec(&HotReloadMessage::UpdatedLibs(paths)) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            let _ = sender.send(Message::Binary(content)).await;
        }
        {
            let asset_dir = state.asset_directory.to_string_lossy().to_string();
            let dir_content = walkdir::WalkDir::new(state.asset_directory.as_path());
            println!("Looking for assets in {asset_dir}");
            let paths = dir_content
                .into_iter()
                .filter_map(|v| v.ok())
                .filter(|v| v.file_type().is_file())
                .map(|v| {
                    let path = v.path().to_path_buf();
                    let result = (
                        path.to_string_lossy().to_string().replace(&asset_dir, ""),
                        path,
                    );
                    println!("Checking {result:?}");
                    result
                })
                .filter_map(|(v, f)| std::fs::read(f).ok().map(|f| (v, f)))
                .map(|(name, f)| {
                    let hash = blake3::hash(&f);

                    (name, hash.as_bytes().to_owned())
                })
                .collect::<Vec<_>>();
            let Ok(content) = rmp_serde::to_vec(&HotReloadMessage::UpdatedAssets(paths)) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            let _ = sender.send(Message::Binary(content)).await;
        }

        {
            let Ok(content) = rmp_serde::to_vec(&HotReloadMessage::RootLibPath(lib_path)) else {
                eprintln!("Couldn't serialize current path - closing connection");
                return;
            };
            let _ = sender.send(Message::Binary(content)).await;
        }
        updates_rx
    };
    while let Ok(msg) = tokio::select! {
       val = updates_rx.recv() => val,
       val = asset_rx.recv() => val,
        _ = tokio::time::sleep(Duration::from_secs(5)) => Ok(HotReloadMessage::KeepAlive)
    } {
        println!("Sending update {msg:?}");
        let Ok(content) = rmp_serde::to_vec(&msg) else {
            eprintln!("Couldn't serialize current updated paths - closing connection");
            continue;
        };
        let _ = sender.send(Message::Binary(content)).await;
    }
}

async fn target_file_loader(
    Path((target, file)): Path<(String, String)>,
    state: State<ServerState>,
    request: Request<Body>,
) -> Result<Response<axum::body::Body>, RequestError> {
    let target = target.parse()?;
    println!("Requested file {file} from {target}");
    let dir = {
        let map = state.map.read().await;
        let target = map
            .get(&target)
            .ok_or(RequestError::FileFromUnbuiltTarget(target, file.clone()))?;
        target.lib_dir.clone()
    };
    let file = dir.join(file);
    println!("File path {file:?}");
    let serve = ServeFile::new(file);
    let result = serve.oneshot(request).await.unwrap();
    println!("Result has status {:?}", result.status());
    Ok(result.into_response())
}

#[derive(Clone)]
pub struct ServerState {
    manager: Arc<Manager>,
}

impl ServerState {
    async fn get_target_connection(
        &self,
        target: &Target,
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
            build_target: Some(*target),
            broadcast_folders: self.broadcast.clone(),
            ..Default::default()
        };

        let (lib_path, lib_dir) =
            broadcast_reloadable(options, sender.clone()).expect("Couldn't setup reloadable");

        let lib_path = lib_path
            .file_name()
            .unwrap_or(lib_path.as_os_str())
            .to_string_lossy()
            .to_string();

        let target_broadcast_info = TargetbroadcastInfo {
            sender: sender.clone(),
            lib_path: lib_path.clone(),
            lib_dir: lib_dir.clone(),
        };
        map.insert(*target, target_broadcast_info);
        (receiver, lib_path, lib_dir)
    }
}

#[derive(Clone)]
struct TargetbroadcastInfo {
    sender: broadcast::Sender<HotReloadMessage>,
    lib_path: String,
    lib_dir: PathBuf,
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Couldn't start listener")]
    SocketListenerError(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum RequestError {
    #[error("Couldn't parse target")]
    TargetParseError(#[from] TargetParseError),
    #[error("Requested file {1} from unbuilt target {0}")]
    FileFromUnbuiltTarget(Target, String),
    #[error("File has invalid encoding")]
    InvalidEncoding(String),
    #[error("Manager failed to process request")]
    ManagerError(#[from] ManagerError),
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response<axum::body::Body> {
        match self {
            RequestError::TargetParseError(error) => Response::builder()
                .status(400)
                .body(axum::body::Body::new("Invalid Target Name".to_string()))
                .unwrap(),
            RequestError::FileFromUnbuiltTarget(t, f) => Response::builder()
                .status(400)
                .body(axum::body::Body::new(format!(
                    "Requested a file {f} from an unbuilt target {t}"
                )))
                .unwrap(),
            RequestError::InvalidEncoding(_) => Response::builder()
                .status(400)
                .body(axum::body::Body::new(
                    "File has invalid encoding".to_string(),
                ))
                .unwrap(),
            RequestError::ManagerError(e) => Response::builder()
                .status(400)
                .body(axum::body::Body::new(e.to_string()))
                .unwrap(),
        }
    }
}

pub fn broadcast_reloadable(
    options: HotReloadOptions,
    update_channel: tokio::sync::broadcast::Sender<HotReloadMessage>,
) -> Result<(std::path::PathBuf, std::path::PathBuf), ServerError> {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    let (mut settings, paths) = setup_build_settings(&options)?;
    let lib_path = settings.lib_path.clone();
    let lib_dir = settings.out_target.clone();

    if !lib_dir.exists() {
        let _ = std::fs::create_dir_all(lib_dir.as_path());
    }

    for dir in paths.iter() {
        if dir.as_path() != lib_dir.as_path() && dir.exists() {
            tracing::trace!("Checking lib path {dir:?}");
            for file in (dir.read_dir()?).flatten() {
                let path = file.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if path.is_file()
                    && (extension == "dll" || extension == "dylib" || extension == "so")
                {
                    let new_file = lib_dir.join(file.file_name());
                    tracing::trace!("Moving {path:?} to {new_file:?}");
                    std::fs::copy(path, new_file)?;
                }
            }
        }
    }

    settings.updated_file_channel = Some(Arc::new(move |msg| {
        let _ = update_channel.send(msg);
    }));

    tokio::spawn(async move {
        first_exec(&settings).expect("Build failed");
        run_broadcaster_with_settings(&settings).expect("Couldn't run broadcaster");
    });
    Ok((lib_path, lib_dir))
}
