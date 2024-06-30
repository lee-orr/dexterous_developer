pub mod macros;
mod types;

#[cfg(not(feature = "hot"))]
mod cold;

#[cfg(feature = "hot")]
mod hot;

pub use types::*;

#[cfg(feature = "hot")]
pub use hot::{HotReloadPlugin, ReloadableAppContents};

#[cfg(not(feature = "hot"))]
pub use cold::*;

#[cfg(feature = "hot")]
pub use dexterous_developer_instance;
