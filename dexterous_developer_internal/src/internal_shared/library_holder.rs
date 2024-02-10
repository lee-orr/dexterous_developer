use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{bail, Context};
use libloading::Library;

use crate::internal_shared::cargo_path_utils;

struct LibraryHolderInner(Option<Library>, PathBuf);

impl Drop for LibraryHolderInner {
    fn drop(&mut self) {
        self.0 = None;
        let _ = std::fs::remove_file(&self.1);
    }
}

impl LibraryHolderInner {
    pub fn new(path: &PathBuf) -> Option<Self> {
        let extension = path.extension();
        let uuid = uuid::Uuid::new_v4();
        let new_path = path.clone();
        let mut new_path = new_path.with_file_name(uuid.to_string());
        let mut archival_path = path.clone();
        if let Some(extension) = extension {
            new_path.set_extension(extension);
            archival_path.set_extension(format!("{}.backup", extension.to_string_lossy()));
        }
        std::fs::copy(path, archival_path).ok()?;
        std::fs::rename(path, &new_path).ok()?;
        crate::logger::debug!("Copied file to new path");

        await_file(10, &new_path);
        let new_path = dunce::canonicalize(new_path).ok()?;

        // SAFETY: Here we are relying on libloading's safety processes for ensuring the Library we receive is properly set up. We expect that library to respect rust ownership semantics because we control it's compilation and know that it is built in rust as well, but the wrappers are unaware so they rely on unsafe.
        match unsafe { libloading::Library::new(&new_path) } {
            Ok(lib) => {
                crate::logger::info!("Loaded library");
                Some(Self(Some(lib), new_path))
            }
            Err(err) => {
                crate::logger::error!("Error loading library - {new_path:?}: {err:?}");

                crate::logger::error!("Search Paths: ");
                for path in cargo_path_utils::dylib_path() {
                    crate::logger::error!("{path:?}");
                }

                None
            }
        }
    }

    pub fn library(&self) -> Option<&Library> {
        self.0.as_ref()
    }

    pub fn call<T>(&self, name: &str, args: &mut T) -> anyhow::Result<()> {
        let Some(lib) = &self.0 else {
            bail!("Library Unavailable")
        };

        crate::logger::debug!("Preparing to call {name}");

        // SAFETY: This should be safe due to relying on rust ownership semantics for passing values between two rust crates. Since we know that the library itself is a rust rather than C library, we know that it will respect a mutable borrow internally.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(&mut T)> =
                lib.get(name.as_bytes())
                    .context(format!("Couldn't load function {name}"))?;
            crate::logger::debug!("Got symbol");
            func(args);
            crate::logger::debug!("Call complete");
        };
        Ok(())
    }
}

fn await_file(iterations: usize, path: &PathBuf) {
    if path.exists() {
        crate::logger::debug!("Validated {path:?} Exists");
        std::thread::sleep(Duration::from_secs_f32(2.0));
        return;
    }
    if iterations > 0 {
        crate::logger::debug!("{path:?} doesn't exist yet...");
        std::thread::sleep(Duration::from_secs_f32(0.5));
        await_file(iterations.saturating_sub(1), path);
    }
}

#[derive(Clone)]
pub struct LibraryHolder(Arc<LibraryHolderInner>);

impl LibraryHolder {
    pub fn new(path: &PathBuf) -> Option<Self> {
        let inner = LibraryHolderInner::new(path)?;
        Some(Self(Arc::new(inner)))
    }
    pub fn library(&self) -> Option<&Library> {
        self.0.library()
    }

    pub fn call<T>(&self, name: &str, args: &mut T) -> anyhow::Result<()> {
        self.0.call(name, args)
    }
}
