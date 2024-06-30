use bevy::{
    log::{debug, info},
    prelude::*,
};
use dexterous_developer_instance::internal::HOT_RELOAD_INFO;

use crate::{
    hot::{
        CleanupReloaded, CleanupSchedules, DeserializeReloadables, OnReloadComplete,
        ReloadableAppCleanupData, ReloadableAppElements, SerializeReloadables, SetupReload,
    },
    ReloadSettings, ReloadableAppContents,
};

use super::super::ReloadableSetup;

#[derive(Resource)]
pub struct InternalHotReload(pub chrono::DateTime<chrono::Local>, pub bool);

#[derive(Resource, Clone, Debug, Default)]
pub struct ReloadableElementList(pub Vec<&'static str>);

pub fn reset_update_frame(mut reload: ResMut<InternalHotReload>) {
    reload.1 = false;
}

pub fn reload(world: &mut World) {
    {
        let input = world.get_resource::<ButtonInput<KeyCode>>();

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

        let info = HOT_RELOAD_INFO
            .get()
            .expect("Hot Reload Info hasn't been set");

        let update_ready = info.update_ready();
        if !update_ready && !manual_reload {
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
        println!("Swapping Libraries");
        info.update();
        debug!("Setup...");
        let _ = world.try_run_schedule(SetupReload);
        debug!("Set Schedules...");
        register_schedules(world);
        if should_serialize {
            debug!("Deserialize...");
            let _ = world.try_run_schedule(DeserializeReloadables);
        }
        if should_run_setups {
            println!("reload complete");
            let _ = world.try_run_schedule(OnReloadComplete);
        }
    }

    {
        let mut internal_state = world.resource_mut::<InternalHotReload>();
        internal_state.0 = chrono::Local::now();
        internal_state.1 = true;
    }
}

pub fn setup_reloadable_app<T: ReloadableSetup>(name: &'static str, world: &mut World) {
    println!("Running Setup For {name}");
    if let Err(e) = setup_reloadable_app_inner(name, world) {
        eprintln!("Reloadable App Error: {e:?}");
        let Some(mut reloadable) = world.get_resource_mut::<ReloadableAppElements>() else {
            return;
        };

        let mut inner_app = ReloadableAppContents::new(name, &mut reloadable);

        T::default_function(&mut inner_app);
    }
}

#[derive(Debug, Clone)]
enum ReloadableSetupCallError {
    CallFailed,
    ReloadableAppContentsMissing,
}

impl std::fmt::Display for ReloadableSetupCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = match self {
            ReloadableSetupCallError::CallFailed => "Library Call Failed",
            ReloadableSetupCallError::ReloadableAppContentsMissing => {
                "No Reloadable App Contents - Called out of order"
            }
        };
        write!(f, "{v}")
    }
}

fn setup_reloadable_app_inner(
    name: &'static str,
    world: &mut World,
) -> Result<(), ReloadableSetupCallError> {
    println!("Setting up reloadables at {name}");

    let lib = HOT_RELOAD_INFO
        .get()
        .expect("Hot Reload Info hasn't been set");

    let Some(mut reloadable) = world.get_resource_mut::<ReloadableAppElements>() else {
        return Err(ReloadableSetupCallError::ReloadableAppContentsMissing);
    };

    let mut inner_app = ReloadableAppContents::new(name, &mut reloadable);

    println!("Calling Inner Function {name}");
    if let Err(e) = lib.call(name, &mut inner_app) {
        eprintln!("Ran Into Error {e}");
        return Err(ReloadableSetupCallError::CallFailed);
    }

    info!("setup for {name} complete");
    Ok(())
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

    for (original, schedule, reloadable_schedule_label) in reloadable.schedule_iter() {
        debug!("Adding {original:?} to schedule");
        inner.labels.insert(reloadable_schedule_label.clone());
        let exists = schedules.insert(schedule);
        if exists.is_none() {
            if let Some(root) = schedules.get_mut(original.clone()) {
                let label = reloadable_schedule_label.clone();
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(label.clone());
                });
            } else {
                let mut root = Schedule::new(original);
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(reloadable_schedule_label.clone());
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

pub fn toggle_reload_mode(
    settings: Option<ResMut<ReloadSettings>>,
    input: Option<Res<ButtonInput<KeyCode>>>,
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
    input: Option<Res<ButtonInput<KeyCode>>>,
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

pub fn dexterous_developer_occured(reload: Res<InternalHotReload>) -> bool {
    reload.1
}
