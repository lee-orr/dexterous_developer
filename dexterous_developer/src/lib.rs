#[cfg(all(feature = "hot_internal", feature = "bevy"))]
mod hot_internal;

#[cfg(feature = "hot")]
mod hot;

#[cfg(all(not(any(feature = "hot", feature = "hot_internal")), feature = "bevy"))]
mod cold;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
mod internal_shared;

mod types;

pub use dexterous_developer_macros::*;

pub use types::*;

#[cfg(feature = "hot")]
pub use hot::*;

#[cfg(all(feature = "hot_internal", feature = "bevy"))]
pub use hot_internal::*;

#[cfg(any(feature = "hot", feature = "hot_internal"))]
pub use internal_shared::*;

#[cfg(all(not(any(feature = "hot", feature = "hot_internal")), feature = "bevy"))]
pub use cold::*;

#[cfg(feature = "bevy")]
mod require_bevy {

    use bevy::{
        app::PluginGroup, app::PluginGroupBuilder, prelude::Plugin, DefaultPlugins, MinimalPlugins,
    };

    pub fn get_default_plugins() -> PluginGroupBuilder {
        DefaultPlugins.build()
    }

    pub fn get_minimal_plugins() -> PluginGroupBuilder {
        MinimalPlugins.build()
    }

    pub trait InitializablePlugins: PluginGroup {
        fn generate_reloadable_initializer() -> PluginGroupBuilder;
    }

    impl InitializablePlugins for DefaultPlugins {
        fn generate_reloadable_initializer() -> PluginGroupBuilder {
            get_default_plugins()
        }
    }
    impl InitializablePlugins for MinimalPlugins {
        fn generate_reloadable_initializer() -> PluginGroupBuilder {
            get_minimal_plugins()
        }
    }

    pub struct InitialPluginsEmpty;

    impl InitialPlugins for InitialPluginsEmpty {
        fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder {
            T::generate_reloadable_initializer()
        }
    }

    impl<P: Plugin> InitialPlugins for P {
        fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder {
            T::generate_reloadable_initializer().add(self)
        }
    }

    pub trait InitialPlugins {
        fn initialize<T: InitializablePlugins>(self) -> PluginGroupBuilder;
    }
}

#[cfg(feature = "bevy")]
pub use require_bevy::*;
