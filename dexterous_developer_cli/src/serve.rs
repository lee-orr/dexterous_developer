use std::{collections::HashMap, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

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
use dexterous_developer_internal::{watch_reloadable, HotReloadMessage, HotReloadOptions, Target};
use futures_util::{SinkExt, StreamExt};
use notify::{RecursiveMode, Watcher};
use tokio::{
    process::Command,
    sync::{
        broadcast::{self, Receiver},
        RwLock,
    },
};
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

use crate::cross::check_cross_requirements_installed;

pub async fn run_server(port: u16, package: Option<String>, features: Vec<String>) -> Result<()> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/libs/:target/:file", get(target_file_loader))
        .route("/connect/:target", get(websocket_connect));

    let asset_directory = std::env::current_dir()?.join("assets");
    if !asset_directory.exists() {
        std::fs::create_dir_all(asset_directory.as_path())?;
    }

    let (asset_tx, asset_rx) = broadcast::channel(100);

    {
        let watch_folder = asset_directory.clone();
        let tx = asset_tx.clone();
        tokio::spawn(async move {
            let mut asset_rx = asset_rx;
            let asset_dir = watch_folder.to_string_lossy().to_string();
            println!("Spawned asset watch thread - {asset_dir}");

            let Ok(mut watcher) =
                notify::recommended_watcher(move |e: notify::Result<notify::Event>| {
                    if let Ok(e) = e {
                        if matches!(
                            e.kind,
                            notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                        ) {
                            let paths = e
                                .paths
                                .into_iter()
                                .filter(|v| v.is_file())
                                .map(|v| {
                                    let path = v;
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
                            let _ = tx.send(HotReloadMessage::UpdatedAssets(paths));
                        }
                    }
                })
            else {
                eprintln!("Couldn't setup watcher");
                return;
            };

            println!("Watching Assets - {watch_folder:?}");
            if let Err(e) = watcher.watch(watch_folder.as_path(), RecursiveMode::Recursive) {
                eprintln!("Error watching files: {e:?}");
            }

            while asset_rx.recv().await.is_ok() {}
            println!("Watcher Exit");
        });
    }

    let app = app.nest_service("/assets", ServeDir::new(asset_directory.as_path()));

    let app = app.with_state(ServerState {
        map: Default::default(),
        package,
        features,
        asset_directory,
        asset_tx,
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
    ws.on_upgrade(move |socket| websocket_connection(socket, target, state))
}

async fn websocket_connection(socket: WebSocket, target: Target, state: State<ServerState>) {
    let (mut sender, _) = socket.split();
    let mut asset_rx = state.asset_tx.subscribe();
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
            let Ok(content) = serde_json::to_string(&HotReloadMessage::UpdatedLibs(paths)) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            let _ = sender.send(Message::Text(content)).await;
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
            let Ok(content) = serde_json::to_string(&HotReloadMessage::UpdatedAssets(paths)) else {
                eprintln!("Couldn't serialize current updated paths - closing connection");
                return;
            };
            println!("Sending Asset List: {content}");
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
    while let Ok(msg) = tokio::select! {
       val = updates_rx.recv() => val,
       val = asset_rx.recv() => val,
        _ = tokio::time::sleep(Duration::from_secs(5)) => Ok(HotReloadMessage::KeepAlive)
    } {
        println!("Sending update {msg:?}");
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
    let target = target.parse()?;
    println!("Requested file {file} from {target}");
    let dir = {
        let map = state.map.read().await;
        let target = map
            .get(&target)
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
    map: Arc<RwLock<HashMap<Target, TargetWatchInfo>>>,
    package: Option<String>,
    features: Vec<String>,
    asset_directory: PathBuf,
    asset_tx: broadcast::Sender<HotReloadMessage>,
}

impl ServerState {
    async fn get_target_connection(
        &self,
        target: &Target,
    ) -> (Receiver<HotReloadMessage>, String, PathBuf) {
        check_cross_requirements_installed(target).expect("Cross Compilation Requirements Missing");

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
        map.insert(*target, target_watch_info);
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
