#[cfg(feature = "hot")]
mod hot;

#[cfg(not(feature = "hot"))]
mod cold;

mod types;

use bevy::{app::PluginGroup, app::PluginGroupBuilder, DefaultPlugins, MinimalPlugins};
pub use reload_macros::*;

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

    pub fn with_default_plugins(self) -> PluginGroupBuilder {
        #[cfg(features = "hot")]
        let initial = initial
            .disable(bevy::winit::WinitPlugin)
            .add(bevy_hot_winit::HotWinitPlugin);

        DefaultPlugins.build().add(self.0)
    }

    pub fn with_minimal_plugins(self) -> PluginGroupBuilder {
        MinimalPlugins.build().add(self.0)
    }
}
