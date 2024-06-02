use safer_ffi::{derive_ReprC, prelude::c_slice};

#[cfg(feature = "dylib")]
pub mod library_holder;

#[derive_ReprC]
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HotReloadInfo {
    internal_last_update_version: extern "C" fn() -> u32,
    internal_update_ready: extern "C" fn() -> bool,
    internal_update: extern "C" fn() -> bool,
    internal_validate_setup: extern "C" fn(u32) -> u32,
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

#[cfg(feature = "hot_internal")]
pub mod internal {
    use camino::Utf8PathBuf;
    use chrono::{Local, Timelike};
    use once_cell::sync::OnceCell;
    use safer_ffi::ffi_export;
    use std::str::Utf8Error;
    use thiserror::Error;

    use crate::{HotReloadInfo, UpdatedAsset};

    pub static HOT_RELOAD_INFO: OnceCell<HotReloadInfo> = OnceCell::new();

    #[ffi_export]
    fn dexterous_developer_internal_set_hot_reload_info(info: HotReloadInfo) {
        let value = Local::now();
        let value = value.nanosecond();
        let validation = (info.internal_validate_setup)(value);
        assert_eq!(value, validation, "Couldn't Validate Hot Reload Connection");
        let _ = HOT_RELOAD_INFO.set(info);
    }

    #[cfg(feature = "dylib")]
    mod dylib {
        use std::sync::{Arc, RwLock};

        use camino::Utf8PathBuf;
        use safer_ffi::ffi_export;
        use tracing::error;

        use crate::{library_holder::LibraryHolder, UpdatedAsset};

        use super::HotReloadAccessError;

        static CURRENT_LIBRARY: RwLock<Option<LibraryHolder>> = RwLock::new(None);
        static UPDATE_CALLBACK: RwLock<Option<Arc<dyn Fn() + Send + Sync>>> = RwLock::new(None);
        static UPDATED_ASSET_CALLBACK: RwLock<Option<Arc<dyn Fn(UpdatedAsset) + Send + Sync>>> =
            RwLock::new(None);

        #[ffi_export]
        fn load_internal_library(path: safer_ffi::String) {
            let path = Utf8PathBuf::from(path.to_string());
            let holder = match LibraryHolder::new(&path, false) {
                Ok(holder) => holder,
                Err(e) => {
                    error!("Failed to load library {path} - {e}");
                    return;
                }
            };

            let mut writer = match CURRENT_LIBRARY.write() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed To Set CurrentLibrary {e}");
                    return;
                }
            };

            *writer = Some(holder);
        }

        #[ffi_export]
        fn update_callback_internal() {
            let current = UPDATE_CALLBACK
                .try_read()
                .map_err(|e| HotReloadAccessError::AtomicError(format!("{e}")));

            if let Ok(current) = current.as_ref() {
                if let Some(current) = current.as_ref() {
                    current();
                }
            }
        }

        #[ffi_export]
        fn update_asset_callback_internal(asset: UpdatedAsset) {
            let current = UPDATED_ASSET_CALLBACK
                .try_read()
                .map_err(|e| HotReloadAccessError::AtomicError(format!("{e}")));

            if let Ok(current) = current.as_ref() {
                if let Some(current) = current.as_ref() {
                    current(asset);
                }
            }
        }

        pub(crate) fn call_dylib<T>(name: &str, args: &mut T) -> Result<(), HotReloadAccessError> {
            let current = CURRENT_LIBRARY
                .try_read()
                .map_err(|e| HotReloadAccessError::AtomicError(format!("{e}")))?;

            match current.as_ref() {
                Some(current) => {
                    current.call(name, args).map_err(|e| {
                        HotReloadAccessError::LibraryError(format!("Couldn't Call {name} - {e:?}"))
                    })?;
                }
                None => {
                    return Err(HotReloadAccessError::LibraryError(
                        "No Library Loaded".to_string(),
                    ))
                }
            }

            Ok(())
        }

        pub(crate) fn update_callback(callback: impl Fn() + Send + Sync + 'static) {
            let mut writer = match UPDATE_CALLBACK.write() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed To Set CurrentLibrary {e}");
                    return;
                }
            };

            *writer = Some(Arc::new(callback));
        }

        pub(crate) fn update_asset_callback(
            callback: impl Fn(UpdatedAsset) + Send + Sync + 'static,
        ) {
            let mut writer = match UPDATED_ASSET_CALLBACK.write() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed To Set CurrentLibrary {e}");
                    return;
                }
            };

            *writer = Some(Arc::new(callback));
        }
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
            let result = (self.internal_update)();
            result
        }

        pub fn call<T>(&self, name: &str, args: &mut T) -> Result<(), HotReloadAccessError> {
            #[cfg(feature = "dylib")]
            dylib::call_dylib(name, args)
        }

        pub fn update_callback(&mut self, callback: impl Fn() + Send + Sync + 'static) {
            #[cfg(feature = "dylib")]
            dylib::update_callback(callback);
        }

        pub fn update_asset_callback(
            &mut self,
            callback: impl Fn(UpdatedAsset) + Send + Sync + 'static,
        ) {
            #[cfg(feature = "dylib")]
            dylib::update_asset_callback(callback);
        }
    }

    #[derive(Error, Debug)]
    pub enum HotReloadAccessError {
        #[error("{0}")]
        LibraryError(String),
        #[error("Couldn't decode error {0}")]
        Utf8Error(#[from] Utf8Error),
        #[error("{0}")]
        AtomicError(String),
    }
}

#[cfg(feature = "hot")]
pub mod hot {

    use crate::HotReloadInfo;

    pub struct HotReloadInfoBuilder {
        pub internal_last_update_version: extern "C" fn() -> u32,
        pub internal_update_ready: extern "C" fn() -> bool,
        pub internal_update: extern "C" fn() -> bool,
        pub internal_validate_setup: extern "C" fn(u32) -> u32,
    }

    impl HotReloadInfoBuilder {
        pub fn build(self) -> HotReloadInfo {
            let HotReloadInfoBuilder {
                internal_last_update_version,
                internal_update_ready,
                internal_update,
                internal_validate_setup,
            } = self;
            HotReloadInfo {
                internal_last_update_version,
                internal_update_ready,
                internal_update,
                internal_validate_setup,
            }
        }
    }
}
