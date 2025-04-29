use camino::{Utf8Path, Utf8PathBuf};
use dashmap::DashMap;
use libloading::Library;
use once_cell::sync::Lazy;
use std::time::Duration;

use dexterous_developer_types::{cargo_path_utils, BuilderTypes};
use thiserror::Error;
use tracing::{debug, error, trace};
use uuid::Uuid;

static LIBRARIES: Lazy<DashMap<Uuid, LibraryHolderInner>> = Lazy::new(Default::default);

struct LibraryHolderInner(Option<Library>, Utf8PathBuf);

impl Drop for LibraryHolderInner {
    fn drop(&mut self) {
        self.0 = None;
        let _ = std::fs::remove_file(&self.1);
    }
}

impl LibraryHolderInner {
    pub fn new(
        path: &Utf8Path,
        use_original: bool,
        builder_type: BuilderTypes,
    ) -> Result<(Self, Uuid), LibraryError> {
        trace!("Loading {path:?}");
        let path = path.to_owned();

        let default_library = match builder_type {
            BuilderTypes::Default => true,
        };

        let uuid = uuid::Uuid::new_v4();
        let path = if default_library || use_original {
            println!("Using Original");
            path
        } else {
            println!("Copying To Temporary File");
            let extension = path.extension();
            let new_path = path.clone();
            let mut new_path = new_path.with_file_name(uuid.to_string());
            let mut archival_path = path.clone();
            if let Some(extension) = extension {
                new_path.set_extension(extension);
                archival_path.set_extension(format!("{}.{uuid}.backup", extension));
            }

            await_file(10, &path);
            println!("Finished Waiting for {path}");

            std::fs::copy(&path, &archival_path)?;
            println!("Copied {path} to {archival_path}");

            std::fs::copy(&path, &new_path)?;
            debug!("Copied file to {path} to {new_path}");
            println!("Renamed {path} to {new_path}");

            await_file(10, &new_path);
            println!("validated {new_path}");
            Utf8PathBuf::try_from(dunce::canonicalize(new_path)?)?
        };

        println!("Loading Library");

        // SAFETY: Here we are relying on libloading's safety processes for ensuring the Library we receive is properly set up. We expect that library to respect rust ownership semantics because we control it's compilation and know that it is built in rust as well, but the wrappers are unaware so they rely on unsafe.
        let library = unsafe { Library::new(path.clone()) };
        match library {
            Ok(lib) => {
                println!("Loaded library");
                Ok((Self(Some(lib), path), uuid))
            }
            Err(err) => {
                eprintln!("Error loading library - {path:?}: {err:?}");

                error!("Search Paths: ");
                for path in cargo_path_utils::dylib_path() {
                    error!("{path:?}");
                }

                Err(err)?
            }
        }
    }

    pub fn call<T>(&self, name: &str, args: T) -> Result<(), LibraryError> {
        let Some(lib) = &self.0 else {
            return Err(LibraryError::LibraryUnavailable(self.1.clone()));
        };

        trace!("Preparing to call {name}");

        // SAFETY: This should be safe due to relying on rust ownership semantics for passing values between two rust crates. Since we know that the library itself is a rust rather than C library, we know that it will respect a mutable borrow internally.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(T)> = lib.get(name.as_bytes())?;
            trace!("Got symbol");
            func(args);
            trace!("Call complete");
        };
        Ok(())
    }

    pub fn call_return<T, R>(&self, name: &str, args: T) -> Result<R, LibraryError> {
        let Some(lib) = &self.0 else {
            return Err(LibraryError::LibraryUnavailable(self.1.clone()));
        };

        trace!("Preparing to call {name}");

        // SAFETY: This should be safe due to relying on rust ownership semantics for passing values between two rust crates. Since we know that the library itself is a rust rather than C library, we know that it will respect a mutable borrow internally.
        let result = unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(T) -> R> =
                lib.get(name.as_bytes())?;
            trace!("Got symbol");
            let result = func(args);
            trace!("Call complete");
            result
        };
        Ok(result)
    }
}

fn await_file(iterations: usize, path: &Utf8PathBuf) {
    if path.exists() {
        println!("Validated {path:?} Exists");
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
pub struct LibraryHolder(Uuid, Utf8PathBuf);

impl LibraryHolder {
    pub fn new(
        path: &Utf8Path,
        use_original: bool,
        builder_type: BuilderTypes,
    ) -> Result<Self, LibraryError> {
        let (inner, uuid) = LibraryHolderInner::new(path, use_original, builder_type)?;
        let path = inner.1.clone();
        LIBRARIES.insert(uuid, inner);
        Ok(Self(uuid, path))
    }

    pub fn path(&self) -> Utf8PathBuf {
        self.1.clone()
    }

    pub fn call<T>(&self, name: &str, args: &mut T) -> Result<(), LibraryError> {
        let Some(inner) = LIBRARIES.get(&self.0) else {
            return Err(LibraryError::MissingUuid);
        };

        inner.call(name, args)
    }

    pub fn call_return<T, R>(&self, name: &str, args: &mut T) -> Result<R, LibraryError> {
        let Some(inner) = LIBRARIES.get(&self.0) else {
            return Err(LibraryError::MissingUuid);
        };

        inner.call_return(name, args)
    }

    pub fn varied_call<T>(&self, name: &str, args: T) -> Result<(), LibraryError> {
        let Some(inner) = LIBRARIES.get(&self.0) else {
            return Err(LibraryError::MissingUuid);
        };
        inner.call(name, args)
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
    #[error("Can't Find Library")]
    MissingUuid,
}
