#[cfg(feature = "hot_internal")]
pub mod internal {
    use camino::Utf8PathBuf;
    use safer_ffi::{derive_ReprC, prelude::c_slice};
    use std::{ffi::c_void, str::Utf8Error};
    use thiserror::Error;

    #[derive_ReprC]
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct HotReloadInfo {
        internal_last_update_version: extern "C" fn() -> u32,
        internal_update_ready: extern "C" fn() -> bool,
        internal_call_on_current: extern "C" fn(c_slice::Ref<u8>, *mut c_void) -> CallResponse,
        internal_set_update_callback: extern "C" fn(extern "C" fn(u32) -> ()),
        internal_set_asset_update_callback: extern "C" fn(extern "C" fn(UpdatedAsset) -> ()),
        internal_update: extern "C" fn() -> bool,
    }

    #[derive_ReprC]
    #[repr(C)]
    struct CallResponse {
        success: bool,
        error: c_slice::Box<u8>,
    }

    #[derive_ReprC]
    #[repr(C)]
    #[derive(Clone)]
    pub struct UpdatedAsset {
        inner_name: c_slice::Box<u8>,
        inner_local_path: c_slice::Box<u8>,
    }

    impl UpdatedAsset {
        pub fn name(&self) -> Result<String, Utf8Error> {
            std::str::from_utf8(self.inner_name.as_slice()).map(|v| v.to_string())
        }

        pub fn local_path(&self) -> Result<Utf8PathBuf, Utf8Error> {
            let path = std::str::from_utf8(&self.inner_local_path)?;
            Ok(Utf8PathBuf::from(path))
        }
    }

    impl HotReloadInfo {
        pub fn update_version(&self) -> u32 {
            (self.internal_last_update_version)()
        }

        pub fn update_ready(&self) -> bool {
            (self.internal_update_ready)()
        }

        pub fn update(&self) -> bool {
            println!("Called Update");
            let result = (self.internal_update)();
            println!("Update Result: {result}");
            result
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

        pub fn update_callback(&mut self, callback: extern "C" fn(u32) -> ()) {
            (self.internal_set_update_callback)(callback);
        }

        pub fn update_asset_callback(&mut self, callback: extern "C" fn(UpdatedAsset) -> ()) {
            (self.internal_set_asset_update_callback)(callback);
        }
    }

    #[derive(Error, Debug)]
    pub enum HotReloadAccessError {
        #[error("{0}")]
        LibraryError(String),
        #[error("Couldn't decode error {0}")]
        Utf8Error(#[from] Utf8Error),
    }
}

#[cfg(feature = "hot")]
pub mod hot {
    use safer_ffi::{derive_ReprC, prelude::c_slice};
    use std::ffi::c_void;

    #[derive_ReprC]
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub struct HotReloadInfo {
        pub internal_last_update_version: extern "C" fn() -> u32,
        pub internal_update_ready: extern "C" fn() -> bool,
        pub internal_call_on_current: extern "C" fn(c_slice::Ref<u8>, *mut c_void) -> CallResponse,
        pub internal_set_update_callback: extern "C" fn(extern "C" fn(u32) -> ()),
        pub internal_update: extern "C" fn() -> bool,
        pub internal_set_asset_update_callback: extern "C" fn(extern "C" fn(UpdatedAsset) -> ()),
    }

    #[derive_ReprC]
    #[repr(C)]
    #[derive(Clone)]
    pub struct UpdatedAsset {
        pub inner_name: c_slice::Box<u8>,
        pub inner_local_path: c_slice::Box<u8>,
    }

    #[derive_ReprC]
    #[repr(C)]
    pub struct CallResponse {
        pub success: bool,
        pub error: c_slice::Box<u8>,
    }
}
