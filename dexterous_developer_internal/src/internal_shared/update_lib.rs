use super::{lib_path_set::LibPathSet, library_holder::LibraryHolder};

pub(crate) fn update_lib(library_paths: &LibPathSet) -> Option<LibraryHolder> {
    crate::logger::trace!("Checking for Library");
    let lib_file_path = library_paths.library_path();
    if lib_file_path.is_file() {
        crate::logger::debug!("Found library {lib_file_path:?}");
        let Some(holder) = LibraryHolder::new(&lib_file_path) else {
            return None;
        };
        crate::logger::debug!("Generated file holder");
        Some(holder)
    } else {
        None
    }
}

#[allow(unused)]
pub(crate) fn get_initial_library(library_paths: &LibPathSet) -> Result<LibraryHolder, String> {
    crate::logger::info!("Looking for lib at {library_paths:?}");

    update_lib(library_paths).ok_or("Couldn't find library".to_string())
}
