use bevy::{
    prelude::{App, OnEnter, OnExit, Plugin, PreStartup, Startup, World},
    winit::WinitPlugin,
};

use crate::{
    clear_marked_system, private, HotReload, HotReloadEvent, OnReloadComplete, ReloadableApp,
    ReloadableElementsSetup,
};

pub struct HotReloadPlugin;

impl Plugin for HotReloadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WinitPlugin)
            .init_resource::<HotReload>()
            .add_event::<HotReloadEvent>()
            .add_systems(PreStartup, run_reload_complete);
    }
}

fn run_reload_complete(world: &mut World) {
    let _ = world.try_run_schedule(OnReloadComplete);
}

pub struct ReloadableAppContents<'a>(&'a mut App);

impl ReloadableElementsSetup for bevy::app::App {
    fn setup_reloadable_elements<T: crate::ReloadableSetup>(&mut self) -> &mut Self {
        {
            let mut contents = ReloadableAppContents(self);
            T::default_function(&mut contents);
        }
        self
    }
}

impl<'a> private::ReloadableAppSealed for ReloadableAppContents<'a> {}

impl<'a> ReloadableApp for ReloadableAppContents<'a> {
    fn add_systems<M, L: bevy::ecs::schedule::ScheduleLabel + Eq + std::hash::Hash + Clone>(
        &mut self,
        schedule: L,
        systems: impl bevy::prelude::IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.0.add_systems(schedule, systems);
        self
    }

    fn insert_replacable_resource<R: crate::ReplacableResource>(&mut self) -> &mut Self {
        self.0.init_resource::<R>();
        self
    }

    fn reset_resource<R: bevy::prelude::Resource + Default>(&mut self) -> &mut Self {
        self.0.init_resource::<R>();
        self
    }

    fn reset_resource_to_value<R: bevy::prelude::Resource + Clone>(
        &mut self,
        value: R,
    ) -> &mut Self {
        self.0.insert_resource(value);
        self
    }

    fn register_replacable_component<C: crate::ReplacableComponent>(&mut self) -> &mut Self {
        self
    }

    fn clear_marked_on_reload<C: bevy::prelude::Component>(&mut self) -> &mut Self {
        self
    }

    fn reset_setup<C: bevy::prelude::Component, M>(
        &mut self,
        systems: impl bevy::prelude::IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.0.add_systems(Startup, systems);
        self
    }

    fn reset_setup_in_state<C: bevy::prelude::Component, S: bevy::prelude::States, M>(
        &mut self,
        state: S,
        systems: impl bevy::prelude::IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.0
            .add_systems(OnEnter(state.clone()), systems)
            .add_systems(OnExit(state), clear_marked_system::<C>);
        self
    }
}
