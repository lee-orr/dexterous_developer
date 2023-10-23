use bevy::ecs::schedule::ScheduleLabel;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SerializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeserializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct CleanupReloaded;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct OnReloadComplete;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ApplyInitialReloadables;
