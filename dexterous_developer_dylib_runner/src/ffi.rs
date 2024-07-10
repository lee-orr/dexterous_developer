use std::sync::{atomic::AtomicU32, Arc};

use camino::Utf8PathBuf;
use crossbeam::atomic::AtomicCell;
use dexterous_developer_instance::library_holder::LibraryHolder;
use once_cell::sync::OnceCell;
use safer_ffi::ffi_export;
use tracing::{error, trace};

use crate::dylib_runner_message::DylibRunnerOutput;

pub static LAST_UPDATE_VERSION: AtomicU32 = AtomicU32::new(0);
pub static NEXT_UPDATE_VERSION: AtomicU32 = AtomicU32::new(0);
pub static ORIGINAL_LIBRARY: OnceCell<Arc<LibraryHolder>> = OnceCell::new();
pub static NEXT_LIBRARY: AtomicCell<Option<Arc<Utf8PathBuf>>> = AtomicCell::new(None);
pub static OUTPUT_SENDER: OnceCell<Arc<async_channel::Sender<DylibRunnerOutput>>> = OnceCell::new();

#[ffi_export]
pub extern "C" fn validate_setup(value: u32) -> u32 {
    value
}

#[ffi_export]
pub extern "C" fn last_update_version() -> u32 {
    LAST_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst)
}

#[ffi_export]
pub extern "C" fn update_ready() -> bool {
    let last = LAST_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    let next = NEXT_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    trace!("Checking Readiness: {last} {next}");
    next > last
}

#[ffi_export]
pub extern "C" fn update() -> bool {
    let next = NEXT_UPDATE_VERSION.load(std::sync::atomic::Ordering::SeqCst);
    let old = LAST_UPDATE_VERSION.swap(next, std::sync::atomic::Ordering::SeqCst);

    if old < next {
        if let Some(path) = NEXT_LIBRARY.take() {
            if let Some(library) = ORIGINAL_LIBRARY.get() {
                if let Err(e) = library.varied_call(
                    "load_internal_library",
                    safer_ffi::String::from(path.as_str()),
                ) {
                    error!("Failed to load library: {e}");
                    return false;
                }

                if let Some(tx) = OUTPUT_SENDER.get() {
                    let _ = tx.send_blocking(DylibRunnerOutput::LoadedLib { build_id: next });
                }
            }
        }
        true
    } else {
        false
    }
}

#[ffi_export]
pub extern "C" fn send_output(value: safer_ffi::Vec<u8>) {
    if let Some(tx) = OUTPUT_SENDER.get() {
        let _ = tx.send_blocking(DylibRunnerOutput::SerializedMessage {
            message: value.to_vec(),
        });
    }
}
