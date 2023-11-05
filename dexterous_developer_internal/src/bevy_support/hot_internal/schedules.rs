use std::sync::Arc;

use bevy::{app::DynEq, ecs::schedule::ScheduleLabel};

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

#[derive(Debug, Clone)]
pub struct DynamicScheduleLabel(
    Arc<dyn ScheduleLabel>,
    bevy::utils::intern::Interned<dyn ScheduleLabel>,
);

impl DynamicScheduleLabel {
    pub fn new(label: impl ScheduleLabel) -> Self {
        let interned = label.intern();
        let arc = Arc::new(label);
        Self(arc, interned)
    }
}

impl std::hash::Hash for DynamicScheduleLabel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.dyn_hash(state);
    }
}

impl PartialEq for DynamicScheduleLabel {
    fn eq(&self, other: &Self) -> bool {
        self.0.dyn_eq(other.0.as_dyn_eq())
    }
}

impl std::cmp::Eq for DynamicScheduleLabel {}

impl ScheduleLabel for DynamicScheduleLabel {
    #[doc = " Clones this `"]
    #[doc = stringify!(ScheduleLabel)]
    #[doc = "`."]
    fn dyn_clone(&self) -> ::std::boxed::Box<dyn ScheduleLabel> {
        self.0.dyn_clone()
    }

    #[doc = " Casts this value to a form where it can be compared with other type-erased values."]
    fn as_dyn_eq(&self) -> &dyn bevy::utils::label::DynEq {
        self.0.as_dyn_eq()
    }

    #[doc = " Feeds this value into the given [`Hasher`]."]
    #[doc = ""]
    #[doc = " [`Hasher`]: std::hash::Hasher"]
    fn dyn_hash(&self, state: &mut dyn ::std::hash::Hasher) {
        self.0.dyn_hash(state)
    }

    #[doc = " Returns an [`Interned`](bevy_utils::intern::Interned) value corresponding to `self`."]
    fn intern(&self) -> bevy::utils::intern::Interned<dyn ScheduleLabel>
    where
        Self: Sized,
    {
        self.1
    }
}
