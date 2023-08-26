#[cfg(all(feature = "hot_internal", feature = "bevy"))]
mod hot_internal;

#[cfg(feature = "hot")]
mod hot;

#[cfg(not(any(feature = "hot", feature = "hot_internal")))]
mod cold;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
pub mod internal_shared;

mod types;

#[cfg(feature = "bevy")]
pub mod bevy_support;
mod logger;

pub use dexterous_developer_macros::*;

pub use types::HotReloadOptions;

#[cfg(feature = "hot")]
pub use hot::run_reloadabe_app;

#[cfg(feature = "bevy")]
pub use bevy_support::*;
