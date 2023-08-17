use std::{path::PathBuf, sync::Arc};

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

        let lib = unsafe { libloading::Library::new(&new_path).ok() }?;
        println!("Loaded library");
        Some(Self(Some(lib), new_path))
    }
    pub fn library(&self) -> Option<&Library> {
        self.0.as_ref()
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
