#[cfg(feature = "hot_internal")]
mod hot_internal;

#[cfg(feature = "hot")]
mod hot;

#[cfg(not(any(feature = "hot", feature = "hot_internal")))]
mod cold;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
mod internal_shared;

mod types;

use bevy::{app::PluginGroup, app::PluginGroupBuilder, DefaultPlugins, MinimalPlugins};
pub use dexterous_developer_macros::*;

pub use types::*;

#[cfg(feature = "hot")]
pub use hot::*;

#[cfg(feature = "hot_internal")]
pub use hot_internal::*;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
pub use internal_shared::*;

#[cfg(not(any(feature = "hot", feature = "hot_internal")))]
pub use cold::*;

#[cfg(not(feature = "hot_internal"))]
pub fn get_default_plugins() -> PluginGroupBuilder {
    DefaultPlugins.build()
}

#[cfg(feature = "hot_internal")]
pub fn get_default_plugins() -> PluginGroupBuilder {
    DefaultPlugins
        .build()
        .disable::<bevy::winit::WinitPlugin>()
        .add(dexterous_developer_bevy_winit::HotWinitPlugin)
}

pub fn get_minimal_plugins() -> PluginGroupBuilder {
    MinimalPlugins.build()
}
