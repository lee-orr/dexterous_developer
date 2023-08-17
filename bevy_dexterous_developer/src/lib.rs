#[cfg(feature = "hot")]
mod hot;

#[cfg(not(feature = "hot"))]
mod cold;

mod types;

use bevy::{app::PluginGroup, app::PluginGroupBuilder, DefaultPlugins, MinimalPlugins};
pub use dexterous_developer_macros::*;

pub use types::*;

#[cfg(feature = "hot")]
pub use hot::*;

#[cfg(not(feature = "hot"))]
pub use cold::*;

pub struct InitialPlugins(HotReloadPlugin);

impl InitialPlugins {
    pub fn new(plugin: HotReloadPlugin) -> Self {
        Self(plugin)
    }

    #[cfg(not(feature = "hot"))]
    pub fn with_default_plugins(self) -> PluginGroupBuilder {
        DefaultPlugins.build().add(self.0)
    }

    #[cfg(feature = "hot")]
    pub fn with_default_plugins(self) -> PluginGroupBuilder {
        DefaultPlugins
            .build()
            .add(self.0)
            .disable::<bevy::winit::WinitPlugin>()
            .add(dexterous_developer_bevy_winit::HotWinitPlugin)
    }

    pub fn with_minimal_plugins(self) -> PluginGroupBuilder {
        MinimalPlugins.build().add(self.0)
    }
}
