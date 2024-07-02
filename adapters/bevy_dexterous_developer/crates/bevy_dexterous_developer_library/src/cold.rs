use super::types::*;
use bevy::{
    prelude::{App, OnEnter, OnExit, Startup},
    state::app::AppExtStates,
};

pub struct ReloadableAppContents<'a>(&'a mut App);

impl ReloadableElementsSetup for bevy::app::App {
    fn setup_reloadable_elements<T: super::ReloadableSetup>(&mut self) -> &mut Self {
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

    fn init_serializable_resource<R: ReplacableType + bevy::prelude::Resource + Default>(
        &mut self,
    ) -> &mut Self {
        self.0.init_resource::<R>();
        self
    }

    fn insert_serializable_resource<R: ReplacableType + bevy::prelude::Resource>(
        &mut self,
        initializer: impl 'static + Send + Sync + Fn() -> R,
    ) -> &mut Self {
        self.0.insert_resource(initializer());
        self
    }

    fn reset_resource<R: bevy::prelude::Resource + Default>(&mut self) -> &mut Self {
        self.0.init_resource::<R>();
        self
    }

    fn reset_resource_to_value<R: bevy::prelude::Resource>(&mut self, value: R) -> &mut Self {
        self.0.insert_resource(value);
        self
    }

    fn register_serializable_component<C: super::ReplacableType + bevy::prelude::Component>(
        &mut self,
    ) -> &mut Self {
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

    fn add_event<T: bevy::prelude::Event>(&mut self) -> &mut Self {
        self.0.add_event::<T>();
        self
    }

    fn init_state<S: bevy::state::state::FreelyMutableState + ReplacableType + Default>(&mut self) -> &mut Self {
        self.0.init_state::<S>();
        self
    }
}
