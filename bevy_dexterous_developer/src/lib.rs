pub mod macros;
mod types;

#[cfg(not(feature = "hot_internal"))]
mod cold;

#[cfg(feature = "hot_internal")]
mod hot_internal;

pub use types::*;

#[cfg(feature = "hot_internal")]
pub use hot_internal::{HotReloadPlugin, ReloadableAppContents};

#[cfg(not(feature = "hot_internal"))]
pub use cold::*;

#[cfg(feature = "hot_internal")]
pub use dexterous_developer_internal;