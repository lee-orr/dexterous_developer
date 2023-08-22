use bevy::{
    prelude::{Commands, Res, ResMut, Schedule, Schedules, World},
    utils::Instant,
};

use crate::internal_shared::update_lib::update_lib;

use super::super::hot_internal::{
    hot_reload_internal::InternalHotReload, schedules::OnReloadComplete, CleanupReloaded,
    DeserializeReloadables, ReloadableAppCleanupData, ReloadableAppContents, ReloadableSchedule,
    SerializeReloadables, SetupReload,
};

use super::super::ReloadableSetup;

pub fn update_lib_system(mut internal: ResMut<InternalHotReload>) {
    internal.updated_this_frame = false;

    if let Some(lib) = update_lib(&internal.libs) {
        println!("Got Update");
        internal.last_lib = internal.library.clone();
        internal.library = Some(lib);
        internal.updated_this_frame = true;
        internal.last_update_time = Instant::now();
    }
}

pub fn reload(world: &mut World) {
    let internal_state = world.resource::<InternalHotReload>();
    if !internal_state.updated_this_frame {
        return;
    }
    println!("Serializing...");
    let _ = world.try_run_schedule(SerializeReloadables);
    println!("Cleanup...");
    let _ = world.try_run_schedule(CleanupReloaded);
    println!("Setup...");
    let _ = world.try_run_schedule(SetupReload);
    println!("Set Schedules...");
    register_schedules(world);
    println!("Deserialize...");
    let _ = world.try_run_schedule(DeserializeReloadables);
    println!("reload complete");
    let _ = world.try_run_schedule(OnReloadComplete);
}

pub fn setup_reloadable_app<T: ReloadableSetup>(name: &'static str, world: &mut World) {
    if let Err(e) = setup_reloadable_app_inner(name, world) {
        eprintln!("Reloadable App Error: {e:?}");
        let Some(mut reloadable) = world.get_resource_mut::<ReloadableAppContents>() else {
            return;
        };
        println!("setup default");

        T::default_function(reloadable.as_mut());
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

fn setup_reloadable_app_inner(
    name: &'static str,
    world: &mut World,
) -> Result<(), ReloadableSetupCallError> {
    println!("Setting up reloadables at {name}");
    let Some(internal_state) = world.get_resource::<InternalHotReload>() else {
        return Err(ReloadableSetupCallError::InternalHotReloadStateMissing);
    };

    println!("got internal reload state");

    let Some(lib) = &internal_state.library else {
        return Err(ReloadableSetupCallError::LibraryHolderNotSet);
    };
    let lib = lib.clone();

    let Some(mut reloadable) = world.get_resource_mut::<ReloadableAppContents>() else {
        return Err(ReloadableSetupCallError::ReloadableAppContentsMissing);
    };

    if let Err(_e) = lib.call(name, reloadable.as_mut()) {
        return Err(ReloadableSetupCallError::CallFailed);
    }

    println!("setup for {name} complete");
    Ok(())
}

pub fn register_schedules(world: &mut World) {
    println!("Reloading schedules");
    let Some(reloadable) = world.remove_resource::<ReloadableAppContents>() else {
        return;
    };
    println!("Has reloadable app");

    let Some(mut schedules) = world.get_resource_mut::<Schedules>() else {
        return;
    };

    println!("Has schedules resource");

    let mut inner = ReloadableAppCleanupData::default();

    for (original, schedule) in reloadable.schedule_iter() {
        let label = ReloadableSchedule::new(original.clone());
        println!("Adding {label:?} to schedule");
        inner.labels.insert(Box::new(label.clone()));
        let exists = schedules.insert(label.clone(), schedule);
        if exists.is_none() {
            if let Some(root) = schedules.get_mut(&original) {
                let label = label.clone();
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(label.clone());
                });
            } else {
                let label = label.clone();
                let mut root = Schedule::new();
                root.add_systems(move |w: &mut World| {
                    let _ = w.try_run_schedule(label.clone());
                });
                schedules.insert(original, root);
            }
        }
    }

    world.insert_resource(inner);
}

pub fn cleanup(
    mut commands: Commands,
    mut schedules: ResMut<Schedules>,
    reloadable: Res<ReloadableAppCleanupData>,
) {
    for schedule in reloadable.labels.iter() {
        println!("Attempting cleanup for {schedule:?}");
        let cleadn = schedules.insert(schedule.clone(), Schedule::default());
        println!(
            "Tried cleaning {schedule:?} was empty: {}",
            cleadn.is_none()
        );
    }
    println!("Cleanup almost complete");

    commands.insert_resource(ReloadableAppContents::default());
    println!("Cleanup complete");
}

pub fn dexterous_developer_occured(reload: Res<InternalHotReload>) -> bool {
    reload.updated_this_frame
}