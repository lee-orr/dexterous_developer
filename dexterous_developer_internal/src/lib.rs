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
    internal_send_output: extern "C" fn(safer_ffi::Vec<u8>),
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
    use rmp_serde::encode::Error;
    use safer_ffi::ffi_export;
    use serde::{de::DeserializeOwned, Serialize};
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
        use serde::de::DeserializeOwned;
        use tracing::error;

        use crate::{library_holder::LibraryHolder, UpdatedAsset};

        use super::{HotReloadAccessError, HOT_RELOAD_INFO};

        static CURRENT_LIBRARY: RwLock<Option<LibraryHolder>> = RwLock::new(None);
        static UPDATE_CALLBACK: RwLock<Option<Arc<dyn Fn() + Send + Sync>>> = RwLock::new(None);
        static UPDATED_ASSET_CALLBACK: RwLock<Option<Arc<dyn Fn(UpdatedAsset) + Send + Sync>>> =
            RwLock::new(None);
        static MESSAGE_CALLBACK: RwLock<Option<Arc<dyn Fn(safer_ffi::Vec<u8>) + Send + Sync>>> =
            RwLock::new(None);

        #[ffi_export]
        fn load_internal_library(path: safer_ffi::String) {
            println!("Called Internal Library");
            let path = Utf8PathBuf::from(path.to_string());
            let holder = match LibraryHolder::new(&path, false) {
                Ok(holder) => holder,
                Err(e) => {
                    eprintln!("Failed to load library {path} - {e}");
                    return;
                }
            };

            let mut writer = match CURRENT_LIBRARY.write() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Failed To Set CurrentLibrary {e}");
                    return;
                }
            };

            *writer = Some(holder);
            println!("Set Current Library");
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

        #[ffi_export]
        fn send_message_to_reloaded_app(message: safer_ffi::Vec<u8>) {
            let current = MESSAGE_CALLBACK
                .try_read()
                .map_err(|e| HotReloadAccessError::AtomicError(format!("{e}")));

            if let Ok(current) = current.as_ref() {
                if let Some(current) = current.as_ref() {
                    current(message);
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
        pub(crate) fn call_return_dylib<T, R>(
            name: &str,
            args: &mut T,
        ) -> Result<R, HotReloadAccessError> {
            let current = CURRENT_LIBRARY
                .try_read()
                .map_err(|e| HotReloadAccessError::AtomicError(format!("{e}")))?;

            let result = match current.as_ref() {
                Some(current) => current.call_return(name, args).map_err(|e| {
                    HotReloadAccessError::LibraryError(format!("Couldn't Call {name} - {e:?}"))
                })?,
                None => {
                    return Err(HotReloadAccessError::LibraryError(
                        "No Library Loaded".to_string(),
                    ))
                }
            };

            Ok(result)
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

        pub(crate) fn register_message_callback<T: DeserializeOwned>(
            callback: impl Fn(T) + Send + Sync + 'static,
        ) {
            let mut writer = match MESSAGE_CALLBACK.write() {
                Ok(w) => w,
                Err(e) => {
                    error!("Failed To Set CurrentLibrary {e}");
                    return;
                }
            };

            *writer = Some(Arc::new(move |value| {
                let deserialized = rmp_serde::from_slice::<T>(&value);
                if let Ok(value) = deserialized {
                    callback(value)
                }
            }));
        }

        pub(crate) fn send_message(value: Vec<u8>) {
            let Some(info) = HOT_RELOAD_INFO.get() else {
                return;
            };
            (info.internal_send_output)(safer_ffi::Vec::from(value));
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
            (self.internal_update)()
        }

        pub fn call<T>(&self, name: &str, args: &mut T) -> Result<(), HotReloadAccessError> {
            #[cfg(feature = "dylib")]
            dylib::call_dylib(name, args)
        }

        pub fn call_dual_param<T>(
            &self,
            name: &str,
            args: &mut T,
        ) -> Result<(), HotReloadAccessError> {
            #[cfg(feature = "dylib")]
            dylib::call_dylib(name, args)
        }

        pub fn call_return<T, R>(
            &self,
            name: &str,
            args: &mut T,
        ) -> Result<R, HotReloadAccessError> {
            #[cfg(feature = "dylib")]
            dylib::call_return_dylib(name, args)
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

        pub fn register_message_callback<T: DeserializeOwned>(
            &mut self,
            callback: impl Fn(T) + Send + Sync + 'static,
        ) {
            #[cfg(feature = "dylib")]
            dylib::register_message_callback(callback);
        }

        pub fn send_message<T: Serialize>(&mut self, value: T) -> Result<(), Error> {
            let value = rmp_serde::to_vec(&value)?;
            #[cfg(feature = "dylib")]
            dylib::send_message(value);
            Ok(())
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
        pub internal_send_output: extern "C" fn(safer_ffi::Vec<u8>),
    }

    impl HotReloadInfoBuilder {
        pub fn build(self) -> HotReloadInfo {
            let HotReloadInfoBuilder {
                internal_last_update_version,
                internal_update_ready,
                internal_update,
                internal_validate_setup,
                internal_send_output,
            } = self;
            HotReloadInfo {
                internal_last_update_version,
                internal_update_ready,
                internal_update,
                internal_validate_setup,
                internal_send_output,
            }
        }
    }
}
