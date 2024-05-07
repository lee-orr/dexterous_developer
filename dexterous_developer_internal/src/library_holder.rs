use camino::{Utf8Path, Utf8PathBuf};
use libloading::Library;
use std::{sync::Arc, time::Duration};

use dexterous_developer_types::cargo_path_utils;
use thiserror::Error;
use tracing::{debug, error, info};

struct LibraryHolderInner(Option<Library>, Utf8PathBuf);

impl Drop for LibraryHolderInner {
    fn drop(&mut self) {
        self.0 = None;
        let _ = std::fs::remove_file(&self.1);
    }
}

impl LibraryHolderInner {
    pub fn new(path: &Utf8Path, use_original: bool) -> Result<Self, LibraryError> {
        info!("Loading {path:?}");
        let path = path.to_owned();
        let path = if use_original {
            info!("Using Original");
            path
        } else {
            info!("Copying To Temporary File");
            let extension = path.extension();
            let uuid = uuid::Uuid::new_v4();
            let new_path = path.clone();
            let mut new_path = new_path.with_file_name(uuid.to_string());
            let mut archival_path = path.clone();
            if let Some(extension) = extension {
                new_path.set_extension(extension);
                archival_path.set_extension(format!("{}.backup", extension));
            }
            std::fs::copy(&path, archival_path)?;
            std::fs::rename(&path, &new_path)?;
            debug!("Copied file to new path");
    
            await_file(10, &new_path);
            Utf8PathBuf::try_from(dunce::canonicalize(new_path)?)?
        };

        info!("Loading Library");
        // SAFETY: Here we are relying on libloading's safety processes for ensuring the Library we receive is properly set up. We expect that library to respect rust ownership semantics because we control it's compilation and know that it is built in rust as well, but the wrappers are unaware so they rely on unsafe.
        match unsafe { libloading::Library::new(&path) } {
            Ok(lib) => {
                info!("Loaded library");
                Ok(Self(Some(lib), path))
            }
            Err(err) => {
                error!("Error loading library - {path:?}: {err:?}");

                error!("Search Paths: ");
                for path in cargo_path_utils::dylib_path() {
                    error!("{path:?}");
                }

                Err(err)?
            }
        }
    }

    pub fn library(&self) -> Option<&Library> {
        self.0.as_ref()
    }

    pub fn call<T>(&self, name: &str, args: T) -> Result<(), LibraryError> {
        let Some(lib) = &self.0 else {
            return Err(LibraryError::LibraryUnavailable(self.1.clone()));
        };

        info!("Preparing to call {name}");

        // SAFETY: This should be safe due to relying on rust ownership semantics for passing values between two rust crates. Since we know that the library itself is a rust rather than C library, we know that it will respect a mutable borrow internally.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(T)> = lib.get(name.as_bytes())?;
            info!("Got symbol");
            func(args);
            info!("Call complete");
        };
        Ok(())
    }
}

fn await_file(iterations: usize, path: &Utf8PathBuf) {
    if path.exists() {
        debug!("Validated {path:?} Exists");
        std::thread::sleep(Duration::from_secs_f32(2.0));
        return;
    }
    if iterations > 0 {
        debug!("{path:?} doesn't exist yet...");
        std::thread::sleep(Duration::from_secs_f32(0.5));
        await_file(iterations.saturating_sub(1), path);
    }
}

#[derive(Clone)]
pub struct LibraryHolder(Arc<LibraryHolderInner>);

impl LibraryHolder {
    pub fn new(path: &Utf8Path, use_original: bool) -> Result<Self, LibraryError> {
        let inner = LibraryHolderInner::new(path, use_original)?;
        Ok(Self(Arc::new(inner)))
    }
    pub fn library(&self) -> Option<&Library> {
        self.0.library()
    }

    pub fn path(&self) -> Utf8PathBuf {
        self.0.1.clone()
    }

    pub fn call<T>(&self, name: &str, args: &mut T) -> Result<(), LibraryError> {
        self.0.call(name, args)
    }

    pub fn varied_call<T>(&self, name: &str, args: T) -> Result<(), LibraryError> {
        self.0.call(name, args)
    }
}

#[derive(Error, Debug)]
pub enum LibraryError {
    #[error("Library {0} Couldn't Be Found")]
    LibraryUnavailable(Utf8PathBuf),
    #[error("Library Error {0}")]
    LibError(#[from] libloading::Error),
    #[error("File System Error {0}")]
    FsError(#[from] std::io::Error),
    #[error("Utf8 Path Error {0}")]
    Utf8PathError(#[from] camino::FromPathBufError),
}