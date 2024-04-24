use std::{ffi::c_void, str::Utf8Error};

use safer_ffi::{derive_ReprC, prelude::c_slice};
use thiserror::Error;

#[cfg(feature = "hot_internal")]
#[derive_ReprC]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HotReloadInfo {
    internal_last_update_version: extern "C" fn() -> u32,
    internal_update_ready: extern "C" fn() -> bool,
    internal_current_expected_version: extern "C" fn () -> u32,
    internal_call_on_current: extern "C" fn(c_slice::Ref<u8>, *mut c_void) -> CallResponse,
    internal_set_update_callback: extern "C" fn(extern "C" fn(u32) -> ()),
    internal_update: extern "C" fn () -> bool
}

#[cfg(feature = "hot_internal")]
#[derive_ReprC]
#[repr(C)]
struct CallResponse {
    success: bool,
    error: c_slice::Box<u8>,
}

#[cfg(feature = "hot_internal")]
impl HotReloadInfo {
    pub fn update_version(&self) -> u32 {
        (self.internal_last_update_version)()
    }

    pub fn update_ready(&self) -> bool {
        let update = self.update_version();
        update > (self.internal_current_expected_version)()
    }

    pub fn update(&self) -> bool {
        (self.internal_update)()
    }

    pub fn call<T>(&self, name: &str, args: &mut T) -> Result<(), HotReloadAccessError> {
        let name = name.as_bytes();
        let name = c_slice::Ref::from(name);
        
        let ptr: *mut T = &mut *args;
        let ptr = ptr.cast::<c_void>();
        let result = (self.internal_call_on_current)(name, ptr);
        if result.success {
            Ok(())
        } else {
            let error = std::str::from_utf8(result.error.as_slice())?;
            Err(HotReloadAccessError::LibraryError(error.to_string()))
        }
    }

    pub fn update_callback(&mut self, callback: extern "C" fn(u32) -> ()) {}
}

#[derive(Error, Debug)]
pub enum HotReloadAccessError {
    #[error("{0}")]
    LibraryError(String),
    #[error("Couldn't decode error {0}")]
    Utf8Error(#[from] Utf8Error),
}
