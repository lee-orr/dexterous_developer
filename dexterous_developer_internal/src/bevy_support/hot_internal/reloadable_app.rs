use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::{
        debug, in_state, info, resource_changed, Commands, Component, Condition, IntoSystemConfigs,
        OnExit, PreUpdate, Resource, Schedule, State, States,
    },
    utils::{HashMap, HashSet},
};

use super::{super::types::*, reload_systems::dexterous_developer_occured};

use super::{replacable_types::*, schedules::*};

#[derive(Default, Resource, Clone, Debug)]
pub struct ReloadableAppCleanupData {
    pub labels: HashSet<Box<dyn ScheduleLabel>>,
}

#[derive(Default, Resource)]
pub struct ReloadableAppContents {
    schedules: HashMap<Box<dyn ScheduleLabel>, Schedule>,
    resources: HashSet<String>,
    components: HashSet<String>,
}

impl ReloadableAppContents {
    pub(crate) fn schedule_iter(self) -> impl Iterator<Item = (Box<dyn ScheduleLabel>, Schedule)> {
        self.schedules.into_iter()
    }
}

impl crate::private::ReloadableAppSealed for ReloadableAppContents {}

impl crate::ReloadableApp for ReloadableAppContents {
    fn add_systems<M, L: ScheduleLabel + Eq + ::std::hash::Hash + Clone>(
        &mut self,
        schedule: L,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let schedules = &mut self.schedules;
        let schedule: Box<dyn ScheduleLabel> = Box::new(schedule);

        if let Some(schedule) = schedules.get_mut(&schedule) {
            debug!("Adding systems to schedule");
            schedule.add_systems(systems);
        } else {
            debug!("Creating schedule with systems");
            let mut new_schedule = Schedule::new();
            new_schedule.add_systems(systems);
            schedules.insert(schedule, new_schedule);
        }

        self
    }

    fn insert_replacable_resource<R: ReplacableResource>(&mut self) -> &mut Self {
        let name = R::get_type_name();
        if !self.resources.contains(name) {
            self.resources.insert(name.to_string());
            info!("adding resource {name}");
            self.add_systems(SerializeReloadables, serialize_replacable_resource::<R>)
                .add_systems(DeserializeReloadables, deserialize_replacable_resource::<R>);
        }
        self
    }

    fn register_replacable_component<C: ReplacableComponent>(&mut self) -> &mut Self {
        let name = C::get_type_name();
        if !self.components.contains(name) {
            self.components.insert(name.to_string());
            self.add_systems(SerializeReloadables, serialize_replacable_component::<C>)
                .add_systems(
                    DeserializeReloadables,
                    deserialize_replacable_component::<C>,
                );
        }
        self
    }

    fn reset_resource<R: Resource + Default>(&mut self) -> &mut Self {
        debug!("resetting resource");
        self.add_systems(DeserializeReloadables, |mut commands: Commands| {
            commands.insert_resource(R::default());
        });
        self
    }

    fn reset_resource_to_value<R: Resource + Clone>(&mut self, value: R) -> &mut Self {
        debug!("resetting resource");
        self.add_systems(DeserializeReloadables, move |mut commands: Commands| {
            commands.insert_resource(value.clone());
        });
        self
    }

    fn clear_marked_on_reload<C: Component>(&mut self) -> &mut Self {
        self.add_systems(CleanupReloaded, clear_marked_system::<C>);
        self
    }

    fn reset_setup<C: Component, M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.add_systems(CleanupReloaded, clear_marked_system::<C>)
            .add_systems(OnReloadComplete, systems)
    }

    fn reset_setup_in_state<C: Component, S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.add_systems(CleanupReloaded, clear_marked_system::<C>)
            .add_systems(OnExit(state.clone()), clear_marked_system::<C>)
            .add_systems(
                PreUpdate,
                systems.run_if(
                    in_state(state).and_then(
                        dexterous_developer_occured.or_else(resource_changed::<State<S>>()),
                    ),
                ),
            )
    }
}
