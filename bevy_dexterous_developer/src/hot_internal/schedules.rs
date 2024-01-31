use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SerializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct DeserializeReloadables;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct SetupReload;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct CleanupReloaded;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct CleanupSchedules;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct OnReloadComplete;

#[derive(ScheduleLabel, Debug, PartialEq, Eq, Hash, Clone)]
pub struct ReloadableSchedule<T: ScheduleLabel>(T);

impl<T: ScheduleLabel> ReloadableSchedule<T> {
    pub fn new(label: T) -> Self {
        Self(label)
    }
}

#[derive(Clone)]
pub struct WrappedSchedule(InternedScheduleLabel, std::sync::Arc<dyn ScheduleLabel>);

impl WrappedSchedule {
    pub fn new(label: impl ScheduleLabel + Clone) -> Self {
        Self(label.clone().intern(), std::sync::Arc::new(label.clone()))
    }
}

impl std::fmt::Debug for WrappedSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ReloadableSchedule").field(&self.1).finish()
    }
}

impl Eq for WrappedSchedule {}

impl PartialEq for WrappedSchedule {
    fn eq(&self, other: &Self) -> bool {
        self.1.eq(&other.1)
    }
}

impl std::hash::Hash for WrappedSchedule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl ScheduleLabel for WrappedSchedule {
    fn dyn_clone(&self) -> ::std::boxed::Box<dyn ScheduleLabel> {
        self.1.dyn_clone()
    }

    fn as_dyn_eq(&self) -> &dyn bevy::utils::label::DynEq {
        self.1.as_dyn_eq()
    }

    fn dyn_hash(&self, state: &mut dyn ::std::hash::Hasher) {
        self.1.dyn_hash(state)
    }

    fn intern(&self) -> InternedScheduleLabel
    where
        Self: Sized,
    {
        self.0.clone()
    }
}
