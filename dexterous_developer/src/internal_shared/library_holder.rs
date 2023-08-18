use std::{path::PathBuf, sync::Arc, time::Duration};

use libloading::Library;

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
        if let Some(extension) = extension {
            new_path.set_extension(extension);
        }
        println!("New path: {new_path:?}");
        std::fs::rename(path, &new_path).ok()?;
        println!("Copied file to new path");

        await_file(3, &new_path);

        // SAFETY: Here we are relying on libloading's safety processes for ensuring the Library we receive is properly set up. We expect that library to respect rust ownership semantics because we control it's compilation and know that it is built in rust as well, but the wrappers are unaware so they rely on unsafe.
        match unsafe { libloading::Library::new(&new_path) } {
            Ok(lib) => {
                println!("Loaded library");
                Some(Self(Some(lib), new_path))
            }
            Err(err) => {
                eprintln!("Error loading library: {err:?}");
                None
            }
        }
    }
    pub fn library(&self) -> Option<&Library> {
        self.0.as_ref()
    }
}

fn await_file(iterations: usize, path: &PathBuf) {
    if path.exists() {
        println!("Validated {path:?} Exists");
        return;
    }
    if iterations > 0 {
        println!("{path:?} doesn't exist yet...");
        await_file(iterations.saturating_sub(1), path);
        std::thread::sleep(Duration::from_secs_f32(0.5));
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
}
