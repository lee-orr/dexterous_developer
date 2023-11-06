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
                use crate::bevy_support::hot_internal::event_sync::EventSync;
                use crate::bevy_support::hot_internal::resource_sync::ResourceSync;
                use bevy::input::{keyboard::*, mouse::*, touch::*, touchpad::*};
                use bevy::window::*;
                use bevy::winit::*;

                group = group.add(ResourceSync::<WinitSettings, _>::bi_directional());
                group = group.add(ComponentSync::<bevy::window::Window, _>::bi_directional());
                group = group
                    .add(ComponentSync::<bevy::window::PrimaryWindow, _>::bi_directional())
                    .add(EventSync::<WindowResized, _>::from_fence())
                    .add(EventSync::<WindowCloseRequested, _>::from_fence())
                    .add(EventSync::<WindowScaleFactorChanged, _>::from_fence())
                    .add(EventSync::<WindowBackendScaleFactorChanged, _>::from_fence())
                    .add(EventSync::<WindowFocused, _>::from_fence())
                    .add(EventSync::<WindowMoved, _>::from_fence())
                    .add(EventSync::<WindowThemeChanged, _>::from_fence())
                    .add(EventSync::<WindowDestroyed, _>::from_fence())
                    .add(EventSync::<KeyboardInput, _>::from_fence())
                    .add(EventSync::<ReceivedCharacter, _>::from_fence())
                    .add(EventSync::<MouseButtonInput, _>::from_fence())
                    .add(EventSync::<TouchpadMagnify, _>::from_fence())
                    .add(EventSync::<TouchpadRotate, _>::from_fence())
                    .add(EventSync::<MouseWheel, _>::from_fence())
                    .add(EventSync::<TouchInput, _>::from_fence())
                    .add(EventSync::<Ime, _>::from_fence())
                    .add(EventSync::<FileDragAndDrop, _>::from_fence())
                    .add(EventSync::<CursorMoved, _>::from_fence())
                    .add(EventSync::<CursorEntered, _>::from_fence())
                    .add(EventSync::<CursorLeft, _>::from_fence())
                    // `winit` `DeviceEvent`s
                    .add(EventSync::<MouseMotion, _>::from_fence());
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
            focused_mode: self.focused_mode,
            unfocused_mode: self.unfocused_mode,
        }
    }

    fn sync_from_app(&self) -> Self {
        Self {
            return_from_run: self.return_from_run,
            focused_mode: self.focused_mode,
            unfocused_mode: self.unfocused_mode,
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
