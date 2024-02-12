use dexterous_developer_types::LibPathSet;
use tracing::{debug, info, trace};

use super::library_holder::LibraryHolder;

pub fn update_lib(library_paths: &LibPathSet) -> Option<LibraryHolder> {
    trace!("Checking for Library");
    let lib_file_path = library_paths.library_path();
    if lib_file_path.is_file() {
        debug!("Found library {lib_file_path:?}");
        let holder = LibraryHolder::new(&lib_file_path)?;
        debug!("Generated file holder");
        Some(holder)
    } else {
        None
    }
}

#[allow(unused)]
pub fn get_initial_library(library_paths: &LibPathSet) -> Result<LibraryHolder, String> {
    info!("Looking for lib at {library_paths:?}");

    update_lib(library_paths).ok_or("Couldn't find library".to_string())
}
