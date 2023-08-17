use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SerializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeserializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SetupReload;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct CleanupReloaded;
#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ReloadableSchedule<T: ScheduleLabel>(T);

impl<T: ScheduleLabel> ReloadableSchedule<T> {
    pub fn new(label: T) -> Self {
        Self(label)
    }
}
