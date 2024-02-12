use chrono::Local;
use dexterous_developer_types::LibPathSet;

use crate::internal_shared::library_holder::LibraryHolder;

pub struct InternalHotReload {
    pub library: Option<LibraryHolder>,
    pub last_lib: Option<LibraryHolder>,
    pub updated_this_frame: bool,
    pub last_update_time: std::time::Instant,
    pub last_update_date_time: chrono::DateTime<Local>,
    pub libs: LibPathSet,
}
