mod types;

#[cfg(not(feature = "hot_internal"))]
mod cold;

mod default_plujgins;
#[cfg(feature = "hot_internal")]
mod hot_internal;

pub use types::*;

#[cfg(feature = "hot_internal")]
pub use hot_internal::{ReloadableAppContents, HotReloadPlugin};

#[cfg(not(feature = "hot_internal"))]
pub use cold::*;
