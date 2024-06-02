#![allow(non_snake_case)]

pub mod dylib_runner_message;
pub mod error;
pub mod ffi;
pub mod remote_connection;

use std::sync::Arc;

use camino::Utf8Path;

use dexterous_developer_internal::{hot::HotReloadInfoBuilder, UpdatedAsset};
use dexterous_developer_types::{cargo_path_utils::dylib_path, Target};
use dylib_runner_message::DylibRunnerMessage;
use error::DylibRunnerError;
use ffi::{NEXT_LIBRARY, NEXT_UPDATE_VERSION, ORIGINAL_LIBRARY};
use safer_ffi::prelude::c_slice;
use tracing::{error, info, warn};

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
                    let result = remote_connection::remote_connection(
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

    let (initial, id, path) = {
        info!("Getting Initial Root");
        let mut library = None;
        let mut id = None;
        #[allow(unused_assignments)]
        let mut path = None;
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
                    path = Some(local_path);
                    id = Some(build_id);
                    break;
                }
                DylibRunnerMessage::AssetUpdated { local_path, name } => {
                    info!("Asset: {name} {local_path}");
                    continue;
                }
            }
        }
        info!("Initial Root ID: {id:?}");
        (
            library.ok_or(DylibRunnerError::NoInitialLibrary)?,
            id.ok_or(DylibRunnerError::NoInitialLibrary)?,
            path.ok_or(DylibRunnerError::NoInitialLibrary)?,
        )
    };

    let initial = Arc::new(initial);

    NEXT_UPDATE_VERSION.store(id, std::sync::atomic::Ordering::SeqCst);
    NEXT_LIBRARY.store(Some(Arc::new(path.clone())));
    ORIGINAL_LIBRARY
        .set(initial.clone())
        .map_err(|_| DylibRunnerError::OnceCellError)?;

    let _handle = std::thread::spawn(|| update_loop(rx, handle));

    info!("Setting Info");

    let info = HotReloadInfoBuilder {
        internal_last_update_version: ffi::last_update_version,
        internal_update_ready: ffi::update_ready,
        internal_update: ffi::update,
        internal_validate_setup: ffi::validate_setup,
    }
    .build();

    initial.varied_call("dexterous_developer_internal_set_hot_reload_info", info)?;

    info!("Calling Internal Main");
    initial.call("dexterous_developer_internal_main", &mut ())?;

    info!("Done.");

    Ok(())
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
                error!("Connection Closed");
                return Ok(());
            }
            DylibRunnerMessage::LoadRootLib {
                build_id,
                local_path,
            } => {
                info!("Load Root New Library {local_path}");
                NEXT_UPDATE_VERSION.store(build_id, std::sync::atomic::Ordering::SeqCst);
                info!("Stored Build ID: {build_id}");
                NEXT_LIBRARY.store(Some(Arc::new(local_path)));
                info!("Stored Library");
                if let Some(library) = ORIGINAL_LIBRARY.get() {
                    info!("Running Callback");
                    let _ = library.varied_call("update_callback_internal", build_id);
                }
            }
            DylibRunnerMessage::AssetUpdated { local_path, name } => {
                if let Some(library) = ORIGINAL_LIBRARY.get() {
                    info!("Running Callback");
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
                    let _ = library.varied_call(
                        "update_asset_callback_internal",
                        UpdatedAsset {
                            inner_name,
                            inner_local_path,
                        },
                    );
                }
            }
        }
    }
}
