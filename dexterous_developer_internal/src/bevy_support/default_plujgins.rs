use bevy::{app::PluginGroupBuilder, prelude::PluginGroup};

use crate::FenceAppSync;

pub(crate) struct FenceDefaultPlugins;

impl PluginGroup for FenceDefaultPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy::core::TaskPoolPlugin::default())
            .add(bevy::core::TypeRegistrationPlugin)
            .add(bevy::core::FrameCountPlugin)
            .add(bevy::time::TimePlugin)
            .add(bevy::diagnostic::DiagnosticsPlugin)
            .add(bevy::input::InputPlugin)
            .add(bevy::window::WindowPlugin::default())
            .add(bevy::a11y::AccessibilityPlugin);

        #[cfg(feature = "bevy_asset")]
        {
            group = group.add(bevy::asset::AssetPlugin::default());
        }

        #[cfg(feature = "bevy_winit")]
        {
            group = group.add(bevy::winit::WinitPlugin::default());

            #[cfg(feature = "hot_internal")]
            {
                use crate::bevy_support::hot_internal::component_sync::ComponentSync;
                use crate::bevy_support::hot_internal::resource_sync::ResourceSync;
                use bevy::winit::WinitSettings;

                group = group.add(ResourceSync::<WinitSettings, _>::bi_directional());
                group = group.add(ComponentSync::<bevy::window::Window, _>::bi_directional());
                group =
                    group.add(ComponentSync::<bevy::window::PrimaryWindow, _>::bi_directional());
            }
        }

        #[cfg(feature = "bevy_audio")]
        {
            group = group.add(bevy::audio::AudioPlugin::default());
        }

        #[cfg(feature = "bevy_gilrs")]
        {
            group = group.add(bevy::gilrs::GilrsPlugin);
        }

        group
    }
}

#[cfg(feature = "bevy_winit")]
impl FenceAppSync<()> for bevy::winit::WinitSettings {
    fn sync_from_fence(&self) -> Self {
        Self {
            return_from_run: self.return_from_run,
            focused_mode: self.focused_mode.clone(),
            unfocused_mode: self.unfocused_mode.clone(),
        }
    }

    fn sync_from_app(&self) -> Self {
        Self {
            return_from_run: self.return_from_run,
            focused_mode: self.focused_mode.clone(),
            unfocused_mode: self.unfocused_mode.clone(),
        }
    }
}

pub(crate) struct HotDefaultPlugins;

impl PluginGroup for HotDefaultPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy::core::TaskPoolPlugin::default())
            .add(bevy::core::TypeRegistrationPlugin)
            .add(bevy::core::FrameCountPlugin)
            .add(bevy::time::TimePlugin)
            .add(bevy::diagnostic::DiagnosticsPlugin)
            .add(bevy::input::InputPlugin)
            .add(bevy::window::WindowPlugin::default())
            .add(bevy::a11y::AccessibilityPlugin);

        #[cfg(feature = "bevy_asset")]
        {
            group = group.add(bevy::asset::AssetPlugin::default());
        }

        #[cfg(feature = "bevy_scene")]
        {
            group = group.add(bevy::scene::ScenePlugin);
        }

        #[cfg(feature = "bevy_render")]
        {
            group = group
                .add(bevy::render::RenderPlugin::default())
                // NOTE: Load this after renderer initialization so that it knows about the supported
                // compressed texture formats
                .add(bevy::render::texture::ImagePlugin::default());

            #[cfg(all(not(target_arch = "wasm32"), feature = "bevy_multi-threaded"))]
            {
                group = group.add(bevy::render::pipelined_rendering::PipelinedRenderingPlugin);
            }

            // NOTE: Load this after renderer initialization so that it knows about the supported
            // compressed texture formats
            #[cfg(feature = "bevy_gltf")]
            {
                group = group.add(bevy::gltf::GltfPlugin::default());
            }
        }

        #[cfg(feature = "bevy_core_pipeline")]
        {
            group = group.add(bevy::core_pipeline::CorePipelinePlugin);
        }

        #[cfg(feature = "bevy_sprite")]
        {
            group = group.add(bevy::sprite::SpritePlugin);
        }

        #[cfg(feature = "bevy_text")]
        {
            group = group.add(bevy::text::TextPlugin);
        }

        #[cfg(feature = "bevy_ui")]
        {
            group = group.add(bevy::ui::UiPlugin);
        }

        #[cfg(feature = "bevy_pbr")]
        {
            group = group.add(bevy::pbr::PbrPlugin::default());
        }

        #[cfg(feature = "bevy_animation")]
        {
            group = group.add(bevy::animation::AnimationPlugin);
        }

        #[cfg(feature = "bevy_gizmos")]
        {
            group = group.add(bevy::gizmos::GizmoPlugin);
        }

        group
    }
}
