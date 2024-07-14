use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::{
    body::Body,
    extract::{
        ws::{self, WebSocket},
        Path, Request, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use camino::Utf8PathBuf;
use dexterous_developer_builder::types::{
    BuildOutputMessages, CurrentBuildState, HashedFileRecord,
};
use dexterous_developer_types::{HotReloadMessage, Target, TargetParseError};
use futures_util::{SinkExt, StreamExt};
use thiserror::Error;
use tokio::sync::broadcast;
use tower::ServiceExt;
use tower_http::services::ServeFile;
use tracing::{error, info, trace};

use crate::{Manager, ManagerError};

pub async fn run_server(port: u16, manager: Manager) -> Result<(), Error> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/target/:target", get(connect_to_target))
        .route("/files/:target/*file", get(target_file_loader));

    let app = app.with_state(ServerState {
        manager: Arc::new(manager),
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let port = listener.local_addr()?.port();

    info!("Listening on http://127.0.0.1:{port}");

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(feature = "test")]
pub async fn run_test_server(
    port: u16,
    manager: Manager,
    port_return: tokio::sync::oneshot::Sender<u16>,
) -> Result<(), Error> {
    let app = Router::new()
        .route("/targets", get(list_targets))
        .route("/target/:target", get(connect_to_target))
        .route("/files/:target/*file", get(target_file_loader));

    let app = app.with_state(ServerState {
        manager: Arc::new(manager),
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let port = listener.local_addr()?.port();

    port_return.send(port).unwrap();

    info!("Listening on http://127.0.0.1:{port}");

    axum::serve(listener, app).await?;
    eprintln!("Ending");

    Ok(())
}

#[derive(Clone)]
pub struct ServerState {
    manager: Arc<Manager>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Server IO Failed {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to serialize {0}")]
    SerdeError(#[from] rmp_serde::encode::Error),
    #[error("Couldn't parse target {0}")]
    TargetParseError(#[from] TargetParseError),
    #[error("Internal Manager Error {0}")]
    ManagerError(#[from] ManagerError),
    #[error("The Impossible Happened {0}")]
    Infallible(#[from] Infallible),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{self}")).into_response()
    }
}

async fn list_targets(state: State<ServerState>) -> Result<Vec<u8>, Error> {
    let targets = state.manager.targets();
    let targets = targets.iter().collect::<Vec<_>>();
    let body_value = rmp_serde::to_vec(&targets)?;
    Ok(body_value)
}

async fn connect_to_target(
    target: Path<String>,
    ws: WebSocketUpgrade,
    state: State<ServerState>,
) -> Result<Response, Error> {
    let id = uuid::Uuid::new_v4();
    trace!("Client {id} Connecting to Target: {target:?}");
    let target: Target = target.0.parse()?;

    let (initial_build_state, builder_rx) = state.manager.watch_target(&target).await?;
    Ok(ws.on_upgrade(move |socket| {
        connected_to_target(id, socket, target, initial_build_state, builder_rx)
    }))
}

async fn connected_to_target(
    id: uuid::Uuid,
    socket: WebSocket,
    _target: Target,
    initial_build_state: CurrentBuildState,
    mut builder_rx: broadcast::Receiver<BuildOutputMessages>,
) {
    info!("Client {id} Connected");
    let (mut ws_sender, _) = socket.split();

    {
        let initial_state_message = HotReloadMessage::InitialState {
            id,
            root_lib: {
                {
                    let lock = initial_build_state.root_library.lock().await;
                    (*lock).as_ref().cloned()
                }
            },
            libraries: initial_build_state
                .libraries
                .iter()
                .map(|asset| (asset.key().clone(), asset.hash))
                .collect(),
            assets: initial_build_state
                .assets
                .iter()
                .map(|asset| (asset.key().clone(), asset.hash))
                .collect(),
            most_recent_started_build: initial_build_state
                .most_recent_started_build
                .load(std::sync::atomic::Ordering::SeqCst),
            most_recent_completed_build: initial_build_state
                .most_recent_completed_build
                .load(std::sync::atomic::Ordering::SeqCst),
            builder_type: initial_build_state.builder_type,
        };
        let Ok(message) = rmp_serde::to_vec(&initial_state_message) else {
            error!("Failed to serialize initial state message for {id}");
            let _ = ws_sender.close().await;
            return;
        };

        if let Err(e) = ws_sender.send(ws::Message::Binary(message)).await {
            error!("Failed to send initial state to {id} - {e}");
            let _ = ws_sender.close().await;
            return;
        }
    }

    while let Ok(msg) = tokio::select! {
        val = builder_rx.recv() => {
            val.map(|msg| match &msg {
                BuildOutputMessages::AssetUpdated(HashedFileRecord {  relative_path, hash, .. }) => Some(HotReloadMessage::UpdatedAssets(relative_path.clone(), *hash)),
                BuildOutputMessages::KeepAlive => None,
                BuildOutputMessages::StartedBuild(id) => Some(HotReloadMessage::BuildStarted(*id)),
                BuildOutputMessages::EndedBuild { id, libraries, root_library } => Some(HotReloadMessage::BuildCompleted {
                    id: *id,
                    libraries: libraries.iter().map(|library| (library.name.clone(), library.hash, library.dependencies.clone())).collect(),
                    root_library: root_library.clone()
                }),
            })
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => Ok(Some(HotReloadMessage::KeepAlive))
    } {
        let Some(msg) = msg else {
            continue;
        };
        let Ok(msg) = rmp_serde::to_vec(&msg) else {
            error!("Failed to serialize update for {id}");
            let _ = ws_sender.close().await;
            return;
        };

        if let Err(e) = ws_sender.send(ws::Message::Binary(msg)).await {
            error!("Failed to send update to {id} - {e}");
            let _ = ws_sender.close().await;
            return;
        }
    }

    info!("Connection closed for {id}");
}

async fn target_file_loader(
    Path((target, file)): Path<(String, Utf8PathBuf)>,
    state: State<ServerState>,
    request: Request<Body>,
) -> Result<Response, Error> {
    let file = Utf8PathBuf::from("./").join(file);
    let target: Target = target.parse()?;
    trace!("Requested file {file:?} from {target}");
    let file = match state.manager.get_filepath(&target, &file) {
        Ok(file) => file,
        Err(e) => {
            error!("Couldn't Find File For Download {e:?}");
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };
    trace!("Found File path: {file:?}");
    let serve = ServeFile::new(file);
    let result = serve.oneshot(request).await?;
    trace!("Result has status {:?}", result.status());
    Ok(result.into_response())
}
