#[cfg(feature = "hot")]
mod hot;

#[cfg(not(any(feature = "hot", feature = "hot_internal")))]
mod cold;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
pub mod internal_shared;

#[cfg(feature = "hot_internal")]
pub mod hot_internal;

pub use dexterous_developer_macros::*;

#[cfg(feature = "hot")]
pub use hot::run_reloadabe_app;

#[cfg(feature = "cli")]
pub use hot::{
    compile_reloadable_libraries, run_existing_library, run_served_file, watch_reloadable,
};
