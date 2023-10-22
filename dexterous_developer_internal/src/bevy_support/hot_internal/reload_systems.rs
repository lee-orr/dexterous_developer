use bevy::{
    log::{debug, error, info},
    prelude::*,
    utils::Instant,
};

use crate::{
    bevy_support::hot_internal::schedules::CleanupSchedules,
    internal_shared::update_lib::update_lib, ReloadSettings,
};

use super::{
    super::hot_internal::{
        hot_reload_internal::InternalHotReload, reloadable_app::ReloadableAppElements,
        schedules::OnReloadComplete, CleanupReloaded, DeserializeReloadables,
        ReloadableAppCleanupData, ReloadableSchedule, SerializeReloadables, SetupReload,
    },
    HotReloadInnerApp,
};

pub fn update_lib_system(mut internal: ResMut<InternalHotReload>) {
    internal.updated_this_frame = false;

    if let Some(lib) = update_lib(&internal.libs) {
        info!("Got Update");
        internal.last_lib = internal.library.clone();
        internal.library = Some(lib);
        internal.updated_this_frame = true;
        internal.last_update_time = Instant::now();
        internal.last_update_date_time = chrono::Local::now();
    }
}

pub fn run_update(inner_app: Option<NonSendMut<HotReloadInnerApp>>) {
    let Some(mut inner_app) = inner_app else {
        return;
    };
    if inner_app.is_ready {
        inner_app.app.update();
    } else {
        inner_app.is_ready = true;
        inner_app.app.run();
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct ReloadableElementList(pub Vec<&'static str>);

pub fn reload(world: &mut World) {
    {
        let internal_state = world.resource::<InternalHotReload>();
        let input = world.get_resource::<Input<KeyCode>>();

        let (reload_mode, manual_reload) = world
            .get_resource::<ReloadSettings>()
            .map(|v| (v.reload_mode, v.manual_reload))
            .unwrap_or_default();

        let manual_reload = if let Some(input) = input {
            manual_reload
                .map(|v| input.just_pressed(v))
                .unwrap_or(false)
        } else {
            false
        };

        if !internal_state.updated_this_frame && !manual_reload {
            return;
        }

        let should_serialize = reload_mode.should_serialize();
        let should_run_setups = reload_mode.should_run_setups();

        if should_serialize {
            debug!("Serializing...");
            let _ = world.try_run_schedule(SerializeReloadables);
        }
        if should_run_setups {
            debug!("Cleanup Reloaded...");
            let _ = world.try_run_schedule(CleanupReloaded);
        }
        debug!("Cleanup Schedules...");
        let _ = world.try_run_schedule(CleanupSchedules);
        debug!("Setup...");
        let _ = world.try_run_schedule(SetupReload);
        debug!("Set Schedules...");
        register_schedules(world);
        if should_serialize {
            debug!("Deserialize...");
            let _ = world.try_run_schedule(DeserializeReloadables);
        }
        if should_run_setups {
            info!("reload complete");
            let _ = world.try_run_schedule(OnReloadComplete);
        }
    }

    {
        let mut internal_state = world.resource_mut::<InternalHotReload>();
        internal_state.last_update_date_time = chrono::Local::now();
    }
}

#[derive(Debug, Clone)]
enum ReloadableSetupCallError {
    InternalHotReloadStateMissing,
    LibraryHolderNotSet,
    CallFailed,
    ReloadableAppContentsMissing,
}

impl std::fmt::Display for ReloadableSetupCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            ReloadableSetupCallError::InternalHotReloadStateMissing => {
                "No Internal Hot Reload Resource Available"
            }
            ReloadableSetupCallError::LibraryHolderNotSet => "Missing a library holder",
            ReloadableSetupCallError::CallFailed => "Library Call Failed",
            ReloadableSetupCallError::ReloadableAppContentsMissing => {
                "No Reloadable App Contents - Called out of order"
            }
        };
        write!(f, "{v}")
    }
}

pub fn register_schedules(world: &mut World) {
    debug!("Reloading schedules");
    let Some(reloadable) = world.remove_resource::<ReloadableAppElements>() else {
        return;
    };
    debug!("Has reloadable app");

    let Some(mut schedules) = world.get_resource_mut::<Schedules>() else {
        return;
    };

    debug!("Has schedules resource");

    let mut inner = ReloadableAppCleanupData::default();

    for (original, schedule) in reloadable.schedule_iter() {
        let label = ReloadableSchedule::new(original.clone());
        debug!("Adding {label:?} to schedule");
        inner.labels.insert(Box::new(label.clone()));
        let exists = schedules.insert(schedule);
        if exists.is_none() {
            if let Some(root) = schedules.get_mut(&original) {
                let label = label.clone();
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(label.clone());
                });
            } else {
                let label = label.clone();
                let mut root = Schedule::new(original);
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(label.clone());
                });
                schedules.insert(root);
            }
        }
    }

    world.insert_resource(inner);
}

pub fn cleanup_schedules(
    mut commands: Commands,
    mut schedules: ResMut<Schedules>,
    reloadable: Res<ReloadableAppCleanupData>,
) {
    for schedule in reloadable.labels.iter() {
        debug!("Attempting cleanup for {schedule:?}");
        let clean = schedules.insert(Schedule::new(schedule.clone()));
        debug!("Tried cleaning {schedule:?} was empty: {}", clean.is_none());
    }
    debug!("Cleanup almost complete");

    commands.insert_resource(ReloadableAppElements::default());
    debug!("Cleanup complete");
}

pub fn dexterous_developer_occured(reload: Res<InternalHotReload>) -> bool {
    reload.updated_this_frame
}

pub fn toggle_reload_mode(
    settings: Option<ResMut<ReloadSettings>>,
    input: Option<Res<Input<KeyCode>>>,
) {
    let Some(input) = input else {
        return;
    };
    let Some(mut settings) = settings else {
        return;
    };

    let Some(toggle) = settings.toggle_reload_mode else {
        return;
    };

    if input.just_pressed(toggle) {
        settings.reload_mode = match settings.reload_mode {
            crate::ReloadMode::Full => crate::ReloadMode::SystemAndSetup,
            crate::ReloadMode::SystemAndSetup => crate::ReloadMode::SystemOnly,
            crate::ReloadMode::SystemOnly => crate::ReloadMode::Full,
        };
    }
}

pub fn toggle_reloadable_elements(
    settings: Option<ResMut<ReloadSettings>>,
    element_list: Option<Res<ReloadableElementList>>,
    input: Option<Res<Input<KeyCode>>>,
) {
    let Some(input) = input else {
        return;
    };
    let Some(mut settings) = settings else {
        return;
    };
    let Some(element_list) = element_list else {
        return;
    };

    let Some((toggle, list)) = (match &settings.reloadable_element_policy {
        crate::ReloadableElementPolicy::All => None,
        crate::ReloadableElementPolicy::OneOfAll(key) => Some((key, element_list.0.as_slice())),
        crate::ReloadableElementPolicy::OneOfList(key, list) => Some((key, list.as_slice())),
    }) else {
        return;
    };

    if input.just_pressed(*toggle) {
        let current = settings.reloadable_element_selection;
        let next = if let Some(current) = current {
            list.iter().skip_while(|v| **v != current).nth(1).copied()
        } else {
            list.first().copied()
        };
        settings.reloadable_element_selection = next;
    }
}
