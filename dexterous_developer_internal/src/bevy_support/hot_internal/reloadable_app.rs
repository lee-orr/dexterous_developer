use bevy::{
    ecs::schedule::common_conditions::run_once,
    ecs::schedule::{run_enter_schedule, ScheduleLabel},
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
    resources: HashMap<&'static str, HashSet<&'static str>>,
    components: HashMap<&'static str, HashSet<&'static str>>,
}

impl ReloadableAppElements {
    pub(crate) fn schedule_iter(self) -> impl Iterator<Item = (Box<dyn ScheduleLabel>, Schedule)> {
        vec![].into_iter()
    }
}

impl crate::private::ReloadableAppSealed for App {}

impl crate::ReloadableApp for App {
    fn register_replacable_resource<R: CustomReplacableResource>(&mut self) -> &mut Self {
        self.init_resource::<R>();

        let name = R::get_type_name();
        let element = R::get_element_label();

        let mut elements = self
            .world
            .get_resource_or_insert_with(ReloadableAppElements::default);

        let mut resource_registry = elements.resources.entry(element).or_default();

        if !resource_registry.contains(name) {
            resource_registry.insert(name);
            info!("adding resource {name}");
            self.add_systems(
                SerializeReloadables,
                serialize_replacable_resource::<R>.run_if(element_selection_condition(element)),
            )
            .add_systems(
                DeserializeReloadables,
                deserialize_replacable_resource::<R>.run_if(element_selection_condition(element)),
            );
        }
        self
    }

    fn register_replacable_component<C: ReplacableComponent>(&mut self) -> &mut Self {
        let name = C::get_type_name();
        let element = C::get_element_label();

        let mut elements = self
            .world
            .get_resource_or_insert_with(ReloadableAppElements::default);

        let mut component_registry = elements.components.entry(element).or_default();

        if !component_registry.contains(name) {
            component_registry.insert(name);
            self.add_systems(
                SerializeReloadables,
                serialize_replacable_component::<C>.run_if(element_selection_condition(element)),
            )
            .add_systems(
                DeserializeReloadables,
                deserialize_replacable_component::<C>.run_if(element_selection_condition(element)),
            );
        }
        self
    }

    fn reset_resource<R: bevy::prelude::Resource + Default + GetElementLabel<L>, L>(
        &mut self,
    ) -> &mut Self {
        debug!("resetting resource");
        let element = R::get_element_label();
        self.add_systems(
            OnReloadComplete,
            (move |mut commands: Commands| {
                commands.insert_resource(R::default());
            })
            .run_if(element_selection_condition(element)),
        );
        self
    }

    fn reset_resource_to_value<R: bevy::prelude::Resource + Clone + GetElementLabel<L>, L>(
        &mut self,
        value: R,
    ) -> &mut Self {
        debug!("resetting resource");
        let element = R::get_element_label();
        self.add_systems(
            OnReloadComplete,
            (move |mut commands: Commands| {
                commands.insert_resource(value.clone());
            })
            .run_if(element_selection_condition(element)),
        );
        self
    }

    fn clear_marked_on_reload<C: bevy::prelude::Component + GetElementLabel<L>, L>(
        &mut self,
    ) -> &mut Self {
        let element = C::get_element_label();
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(element)),
        );
        self
    }

    fn reset_setup<C: bevy::prelude::Component + GetElementLabel<L>, M, L>(
        &mut self,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let element = C::get_element_label();
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(element)),
        )
        .add_systems(
            OnReloadComplete,
            systems.run_if(element_selection_condition(element)),
        )
    }

    fn reset_setup_in_state<
        C: bevy::prelude::Component + GetElementLabel<L>,
        S: bevy::prelude::States,
        M,
        L,
    >(
        &mut self,
        state: S,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        let element = C::get_element_label();
        self.add_systems(
            CleanupReloaded,
            clear_marked_system::<C>.run_if(element_selection_condition(element)),
        )
        .add_systems(OnExit(state.clone()), clear_marked_system::<C>)
        .add_systems(
            PreUpdate,
            systems.run_if(
                in_state(state).and_then(
                    dexterous_developer_occured
                        .and_then(element_selection_condition(element))
                        .or_else(resource_changed::<State<S>>()),
                ),
            ),
        )
    }

    fn add_reloadable_state<S: ReplacableState>(&mut self) -> &mut Self {
        self.register_replacable_resource::<State<S>>()
            .register_replacable_resource::<NextState<S>>()
            .add_systems(
                StateTransition,
                ((
                    run_enter_schedule::<S>.run_if(run_once()),
                    apply_state_transition::<S>,
                )
                    .chain(),),
            );

        self
    }

    fn add_reloadable_event<T: ReplacableEvent>(&mut self) -> &mut Self {
        self.register_replacable_resource::<Events<T>>()
            .add_systems(
                First,
                bevy::ecs::event::event_update_system::<T>
                    .run_if(bevy::ecs::event::event_update_condition::<T>),
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
