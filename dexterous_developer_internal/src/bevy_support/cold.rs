use super::types::*;
use bevy::prelude::{App, OnEnter, OnExit, Startup};

impl private::ReloadableAppSealed for App {}

impl ReloadableApp for App {
    fn register_replacable_resource<R: super::CustomReplacableResource>(&mut self) -> &mut Self {
        self.init_resource::<R>();
        self
    }

    fn reset_resource<R: bevy::prelude::Resource + Default + GetElementLabel<L>, L>(
        &mut self,
    ) -> &mut Self {
        self.init_resource::<R>();
        self
    }

    fn reset_resource_to_value<R: bevy::prelude::Resource + Clone + GetElementLabel<L>, L>(
        &mut self,
        value: R,
    ) -> &mut Self {
        self.insert_resource(value);
        self
    }

    fn register_replacable_component<C: super::ReplacableComponent>(&mut self) -> &mut Self {
        self
    }

    fn clear_marked_on_reload<C: bevy::prelude::Component + GetElementLabel<L>, L>(
        &mut self,
    ) -> &mut Self {
        self
    }

    fn reset_setup<C: bevy::prelude::Component + GetElementLabel<L>, M, L>(
        &mut self,
        systems: impl bevy::prelude::IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.add_systems(Startup, systems);
        self
    }

    fn reset_setup_in_state<
        C: bevy::prelude::Component + GetElementLabel<L>,
        S: bevy::prelude::States,
        M,
        L,
    >(
        &mut self,
        state: S,
        systems: impl bevy::prelude::IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.add_systems(OnEnter(state.clone()), systems)
            .add_systems(OnExit(state), clear_marked_system::<C>);
        self
    }

    fn add_reloadable_event<T: ReplacableEvent>(&mut self) -> &mut Self {
        self.add_event::<T>();
        self
    }

    fn add_reloadable_state<S: super::ReplacableState>(&mut self) -> &mut Self {
        self.add_state::<S>();
        self
    }
}
