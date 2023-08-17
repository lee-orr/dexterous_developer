use crate::hot::{lib_set::LibPathSet, library_holder::LibraryHolder};

pub(crate) fn update_lib(library_paths: &LibPathSet) -> Option<LibraryHolder> {
    let lib_file_path = library_paths.library_path();

    if lib_file_path.is_file() {
        println!("Found library {lib_file_path:?}");
        let Some(holder) = LibraryHolder::new(&lib_file_path) else {
            return None;
        };
        println!("Generated file holder");
        Some(holder)
    } else {
        None
    }
}

pub(crate) fn get_initial_library(library_paths: &LibPathSet) -> LibraryHolder {
    loop {
        if let Some(library) = update_lib(library_paths) {
            println!("Update Thread: {:?}", std::thread::current().id());
            println!("Updated lib");
            return library;
        }
    }
}
