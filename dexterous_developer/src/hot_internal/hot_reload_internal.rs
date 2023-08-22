use crate::internal_shared::{lib_path_set::LibPathSet, library_holder::LibraryHolder};

pub struct InternalHotReload {
    pub library: Option<LibraryHolder>,
    pub last_lib: Option<LibraryHolder>,
    pub updated_this_frame: bool,
    pub last_update_time: std::time::Instant,
    pub libs: LibPathSet,
}
