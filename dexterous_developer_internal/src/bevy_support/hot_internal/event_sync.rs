use std::marker::PhantomData;

use bevy::prelude::*;

use crate::FenceAppSync;

use super::{
    schedules::{SyncFromApp, SyncFromFence},
    HotReloadInnerApp,
};

pub struct EventSync<T: Event + FenceAppSync<M>, M: Send + Sync + 'static>(
    bool,
    bool,
    PhantomData<(T, M)>,
);

impl<T: Event + FenceAppSync<M>, M: Send + Sync + 'static> EventSync<T, M> {
    pub fn from_fence() -> Self {
        Self(true, false, PhantomData)
    }
    pub fn from_app() -> Self {
        Self(false, true, PhantomData)
    }

    pub fn bi_directional() -> Self {
        Self(true, true, PhantomData)
    }
}

impl<T: Event + FenceAppSync<M>, M: Send + Sync + 'static> Plugin for EventSync<T, M> {
    fn build(&self, app: &mut App) {
        if self.0 {
            app.add_systems(SyncFromFence, sync_from_fence::<T, M>);
        }
        if self.1 {
            app.add_systems(SyncFromApp, sync_from_app::<T, M>);
        }
    }
}

fn sync_from_fence<T: Event + FenceAppSync<M>, M: Send + Sync + 'static>(
    mut reader: EventReader<T>,
    mut inner: NonSendMut<HotReloadInnerApp>,
) {
    let Some(inner) = inner.get_app_mut() else {
        return;
    };
    inner
        .world
        .send_event_batch(reader.iter().map(|v| v.sync_from_fence()));
}

fn sync_from_app<T: Event + FenceAppSync<M>, M: Send + Sync + 'static>(
    mut writer: EventWriter<T>,
    inner: NonSend<HotReloadInnerApp>,
) {
    let Some(inner) = inner.get_app() else {
        return;
    };
    let Some(events) = inner.world.get_resource::<Events<T>>() else {
        return;
    };
    let mut reader = events.get_reader();

    writer.send_batch(reader.read(events).map(|v| v.sync_from_app()));
}
