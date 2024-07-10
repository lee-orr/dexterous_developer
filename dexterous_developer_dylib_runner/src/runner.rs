use std::sync::Arc;

use camino::Utf8Path;

use dexterous_developer_instance::{runner::HotReloadInfoBuilder, UpdatedAsset};
use dexterous_developer_types::cargo_path_utils::dylib_path;
use dylib_runner_message::DylibRunnerMessage;
use error::DylibRunnerError;
use ffi::{NEXT_LIBRARY, NEXT_UPDATE_VERSION, ORIGINAL_LIBRARY};
use remote_connection::connect_to_server;
use safer_ffi::prelude::c_slice;
use tracing::{error, info, warn};

use dexterous_developer_instance::library_holder::LibraryHolder;

use crate::{
    dylib_runner_message::{self, DylibRunnerOutput},
    error,
    ffi::{self, OUTPUT_SENDER},
    remote_connection,
};

pub fn run_reloadable_app(
    working_directory: &Utf8Path,
    library_path: &Utf8Path,
    server: url::Url,
    in_workspace: bool,
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

    run_app(|tx, _| {
        connect_to_server(
            working_directory,
            library_path,
            server.clone(),
            tx,
            in_workspace,
        )
    })
}

pub fn run_app<
    T: Fn(
        async_channel::Sender<DylibRunnerMessage>,
        async_channel::Receiver<DylibRunnerOutput>,
    ) -> Result<std::thread::JoinHandle<Result<(), DylibRunnerError>>, DylibRunnerError>,
>(
    connect: T,
) -> Result<(), DylibRunnerError> {
    let (tx, rx) = async_channel::unbounded::<DylibRunnerMessage>();
    let (out_tx, out_rx) = async_channel::unbounded::<DylibRunnerOutput>();

    let handle = connect(tx, out_rx)?;

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
            warn!("Got Message While Looking For Root - {initial:?}");
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
                DylibRunnerMessage::SerializedMessage { message: _ } => {}
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
    OUTPUT_SENDER
        .set(Arc::new(out_tx.clone()))
        .map_err(|_| DylibRunnerError::OnceCellError)?;

    let _handle = std::thread::spawn(|| update_loop(rx, handle));

    info!("Setting Info");

    let info = HotReloadInfoBuilder {
        internal_last_update_version: ffi::last_update_version,
        internal_update_ready: ffi::update_ready,
        internal_update: ffi::update,
        internal_validate_setup: ffi::validate_setup,
        internal_send_output: ffi::send_output,
    }
    .build();

    initial.varied_call("dexterous_developer_instance_set_hot_reload_info", info)?;
    let _ = out_tx.send_blocking(DylibRunnerOutput::LoadedLib { build_id: id });
    info!("Calling Internal Main");
    initial.call("dexterous_developer_instance_main", &mut ())?;

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
            DylibRunnerMessage::SerializedMessage { message } => {
                if let Some(library) = ORIGINAL_LIBRARY.get() {
                    info!("Sending Message");
                    let _ = library.varied_call(
                        "send_message_to_reloaded_app",
                        safer_ffi::Vec::from(message),
                    );
                }
            }
        }
    }
}
