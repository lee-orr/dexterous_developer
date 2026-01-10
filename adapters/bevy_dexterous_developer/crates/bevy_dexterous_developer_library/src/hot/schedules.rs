use bevy::ecs::schedule::{InternedScheduleLabel, ScheduleLabel};
use std::hash::Hash;

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

/// A wrapper around a schedule label that uses string-based comparison.
/// This is crucial for hot reload because InternedScheduleLabel values are
/// interned per-library and don't match after a library swap.
#[derive(Clone)]
pub struct WrappedSchedule {
    interned: InternedScheduleLabel,
    label: std::sync::Arc<dyn ScheduleLabel>,
    /// String representation for stable hash/eq across library swaps
    name: String,
}

impl WrappedSchedule {
    pub fn new(label: impl ScheduleLabel + Clone) -> Self {
        let name = format!("{:?}", label);
        Self {
            interned: label.clone().intern(),
            label: std::sync::Arc::new(label.clone()),
            name,
        }
    }

    /// Returns the inner interned schedule label (the original, unwrapped label)
    pub fn inner(&self) -> InternedScheduleLabel {
        self.interned
    }

    /// Returns the stable string name for this schedule
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Debug for WrappedSchedule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ReloadableSchedule").field(&self.label).finish()
    }
}

impl Eq for WrappedSchedule {}

impl PartialEq for WrappedSchedule {
    fn eq(&self, other: &Self) -> bool {
        // Use string comparison for stable equality across library swaps
        self.name == other.name
    }
}

impl std::hash::Hash for WrappedSchedule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Use string hash for stable hashing across library swaps
        self.name.hash(state);
    }
}

// DynEq and DynHash are automatically implemented via blanket impls
// since WrappedSchedule implements Eq and Hash

impl ScheduleLabel for WrappedSchedule {
    fn dyn_clone(&self) -> ::std::boxed::Box<dyn ScheduleLabel> {
        Box::new(self.clone())
    }
}
