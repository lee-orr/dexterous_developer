use std::marker::PhantomData;

use bevy::{ecs::component::Tick, prelude::*};

use crate::FenceAppSync;

use super::{
    schedules::{SyncFromApp, SyncFromFence},
    HotReloadInnerApp,
};

pub struct ResourceSync<T: Resource + FenceAppSync<M>, M: Send + Sync + 'static>(
    bool,
    bool,
    PhantomData<(T, M)>,
);

impl<T: Resource + FenceAppSync<M>, M: Send + Sync + 'static> ResourceSync<T, M> {
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

impl<T: Resource + FenceAppSync<M>, M: Send + Sync + 'static> Plugin for ResourceSync<T, M> {
    fn build(&self, app: &mut App) {
        app.insert_resource(ResourceTracker(Tick::new(0), PhantomData::<T>));
        if self.0 {
            app.add_systems(SyncFromFence, sync_from_fence::<T, M>);
        }
        if self.1 {
            app.add_systems(SyncFromApp, sync_from_app::<T, M>);
        }
    }
}

#[derive(Resource)]
struct ResourceTracker<T: Resource>(Tick, PhantomData<T>);

fn sync_from_fence<T: Resource + FenceAppSync<M>, M: Send + Sync + 'static>(
    res: Option<Res<T>>,
    mut tracker: ResMut<ResourceTracker<T>>,
    mut inner: NonSendMut<HotReloadInnerApp>,
) {
    let Some(inner) = inner.get_app_mut() else {
        return;
    };
    let Some(res) = res else {
        return;
    };
    if res.is_changed() {
        let world_tick = inner.world.change_tick();
        if world_tick.is_newer_than(tracker.0, world_tick) {
            inner.world.insert_resource(res.sync_from_fence());
            tracker.0 = world_tick;
        }
    }
}

fn sync_from_app<T: Resource + FenceAppSync<M>, M: Send + Sync + 'static>(
    mut commands: Commands,
    mut tracker: ResMut<ResourceTracker<T>>,
    inner: NonSend<HotReloadInnerApp>,
) {
    let Some(inner) = inner.get_app() else {
        return;
    };
    let world_tick = &inner.world.last_change_tick().clone();

    if !inner.world.is_resource_changed::<T>() {
        return;
    }

    let Some(r) = inner.world.get_resource::<T>() else {
        return;
    };
    if world_tick.is_newer_than(tracker.0, *world_tick) {
        commands.insert_resource(r.sync_from_app());
        tracker.0 = *world_tick;
    }
}
