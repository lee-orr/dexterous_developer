use bevy::{
    ecs::schedule::ScheduleLabel,
    prelude::*,
    utils::{HashMap, HashSet},
};

use super::{super::types::*, reload_systems::dexterous_developer_occured};

use super::{replacable_types::*, schedules::*};

#[derive(Default, Resource, Clone, Debug)]
pub struct ReloadableAppCleanupData {
    pub labels: HashSet<Box<dyn ScheduleLabel>>,
}

#[derive(Default, Resource)]
pub struct ReloadableAppElements {
    schedules: HashMap<Box<dyn ScheduleLabel>, Schedule>,
    resources: HashSet<String>,
    components: HashSet<String>,
}

impl ReloadableAppElements {
    pub(crate) fn schedule_iter(self) -> impl Iterator<Item = (Box<dyn ScheduleLabel>, Schedule)> {
        self.schedules.into_iter()
    }
}

pub struct ReloadableAppContents<'a> {
    name: &'static str,
    schedules: &'a mut HashMap<Box<dyn ScheduleLabel>, Schedule>,
    resources: &'a mut HashSet<String>,
    components: &'a mut HashSet<String>,
}

impl<'a> ReloadableAppContents<'a> {
    pub fn new(name: &'static str, elements: &'a mut ReloadableAppElements) -> Self {
        Self {
            name,
            schedules: &mut elements.schedules,
            resources: &mut elements.resources,
            components: &mut elements.components,
        }
    }
}

impl<'a> crate::private::ReloadableAppSealed for ReloadableAppContents<'a> {}

impl<'a> crate::ReloadableApp for ReloadableAppContents<'a> {
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
            let reloadable_element_name = self.name;
            self.add_systems(
                SerializeReloadables,
                serialize_replacable_resource::<R>
                    .run_if(element_selection_condition(reloadable_element_name)),
            )
            .add_systems(
                DeserializeReloadables,
                deserialize_replacable_resource::<R>
                    .run_if(element_selection_condition(reloadable_element_name)),
            );
        }
        self
    }

    fn register_replacable_component<C: ReplacableComponent>(&mut self) -> &mut Self {
        let name = C::get_type_name();
        if !self.components.contains(name) {
            self.components.insert(name.to_string());
            let reloadable_element_name = self.name;
            self.add_systems(
                SerializeReloadables,
                serialize_replacable_component::<C>
                    .run_if(element_selection_condition(reloadable_element_name)),
            )
            .add_systems(
                DeserializeReloadables,
                deserialize_replacable_component::<C>
                    .run_if(element_selection_condition(reloadable_element_name)),
            );
        }
        self
    }

    fn reset_resource<R: Resource + Default>(&mut self) -> &mut Self {
        debug!("resetting resource");
        let name = self.name;
        self.add_systems(
            OnReloadComplete,
            (move |mut commands: Commands| {
                commands.insert_resource(R::default());
            })
            .run_if(element_selection_condition(name)),
        );
        self
    }

    fn reset_resource_to_value<R: Resource + Clone>(&mut self, value: R) -> &mut Self {
        debug!("resetting resource");
        let name = self.name;
        self.add_systems(
            OnReloadComplete,
            (move |mut commands: Commands| {
                commands.insert_resource(value.clone());
            })
            .run_if(element_selection_condition(name)),
        );
        self
    }

    fn clear_marked_on_reload<C: Component>(&mut self) -> &mut Self {
        let name = self.name;
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(name)),
        );
        self
    }

    fn reset_setup<C: Component, M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        let name = self.name;
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(name)),
        )
        .add_systems(
            OnReloadComplete,
            systems.run_if(element_selection_condition(name)),
        )
    }

    fn reset_setup_in_state<C: Component, S: States, M>(
        &mut self,
        state: S,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let name = self.name;
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(name)),
        )
        .add_systems(OnExit(state.clone()), clear_marked_system::<C>)
        .add_systems(
            PreUpdate,
            systems.run_if(
                in_state(state).and_then(
                    dexterous_developer_occured
                        .and_then(element_selection_condition(name))
                        .or_else(resource_changed::<State<S>>()),
                ),
            ),
        )
    }
}

fn element_selection_condition(name: &'static str) -> impl Fn(Option<Res<ReloadSettings>>) -> bool {
    move |settings| {
        if let Some(settings) = settings {
            if let Some(current) = settings.reloadable_element_selection {
                if current != name {
                    return false;
                }
            }
        }
        true
    }
}
