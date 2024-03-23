use std::{sync::Arc, time::Duration};

use anyhow::{bail, Context};
use camino::{Utf8Path, Utf8PathBuf};
use libloading::Library;

use dexterous_developer_types::cargo_path_utils;
use tracing::{debug, error, info};

struct LibraryHolderInner(Option<Library>, Utf8PathBuf);

impl Drop for LibraryHolderInner {
    fn drop(&mut self) {
        self.0 = None;
        let _ = std::fs::remove_file(&self.1);
    }
}

impl LibraryHolderInner {
    pub fn new(path: &Utf8Path) -> Option<Self> {
        let path = path.to_owned();
        let extension = path.extension();
        let uuid = uuid::Uuid::new_v4();
        let new_path = path.clone();
        let mut new_path = new_path.with_file_name(uuid.to_string());
        let mut archival_path = path.clone();
        if let Some(extension) = extension {
            new_path.set_extension(extension);
            archival_path.set_extension(format!("{}.backup", extension));
        }
        std::fs::copy(&path, archival_path).ok()?;
        std::fs::rename(&path, &new_path).ok()?;
        debug!("Copied file to new path");

        await_file(10, &new_path);
        let new_path = Utf8PathBuf::try_from(dunce::canonicalize(new_path).ok()?).ok()?;

        // SAFETY: Here we are relying on libloading's safety processes for ensuring the Library we receive is properly set up. We expect that library to respect rust ownership semantics because we control it's compilation and know that it is built in rust as well, but the wrappers are unaware so they rely on unsafe.
        match unsafe { libloading::Library::new(&new_path) } {
            Ok(lib) => {
                info!("Loaded library");
                Some(Self(Some(lib), new_path))
            }
            Err(err) => {
                error!("Error loading library - {new_path:?}: {err:?}");

                error!("Search Paths: ");
                for path in cargo_path_utils::dylib_path() {
                    error!("{path:?}");
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

        debug!("Preparing to call {name}");

        // SAFETY: This should be safe due to relying on rust ownership semantics for passing values between two rust crates. Since we know that the library itself is a rust rather than C library, we know that it will respect a mutable borrow internally.
        unsafe {
            let func: libloading::Symbol<unsafe extern "C" fn(&mut T)> =
                lib.get(name.as_bytes())
                    .context(format!("Couldn't load function {name}"))?;
            debug!("Got symbol");
            func(args);
            debug!("Call complete");
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
    pub fn new(path: &Utf8Path) -> Option<Self> {
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
