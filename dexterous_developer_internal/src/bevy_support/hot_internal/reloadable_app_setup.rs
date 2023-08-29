use bevy::prelude::World;

use super::super::{ReloadableElementsSetup, ReloadableSetup};

use super::{reload_systems::setup_reloadable_app, schedules::SetupReload};

impl ReloadableElementsSetup for bevy::app::App {
    fn setup_reloadable_elements<T: ReloadableSetup>(&mut self) -> &mut Self {
        let name = T::setup_function_name();
        let system = move |world: &mut World| setup_reloadable_app::<T>(name, world);
        self.add_systems(SetupReload, system)
    }
}
