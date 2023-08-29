use bevy::prelude::Resource;

pub use crate::hot_internal::hot_reload_internal::InternalHotReload;

impl Resource for InternalHotReload {}
