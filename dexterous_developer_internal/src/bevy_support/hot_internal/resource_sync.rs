use std::marker::PhantomData;

use bevy::{ecs::component::Tick, prelude::*};

use super::HotReloadInnerApp;

pub struct ResourceSync<T: Resource + Clone>(bool, bool, PhantomData<T>);

impl<T: Resource + Clone> ResourceSync<T> {
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

impl<T: Resource + Clone> Plugin for ResourceSync<T> {
    fn build(&self, app: &mut App) {
        app.insert_resource(ResourceTracker(Tick::new(0), PhantomData::<T>));
        if self.0 {
            app.add_systems(PreUpdate, sync_from_fence::<T>);
        }
        if self.1 {
            app.add_systems(PostUpdate, sync_from_app::<T>);
        }
    }
}

#[derive(Resource)]
struct ResourceTracker<T: Resource>(Tick, PhantomData<T>);

fn sync_from_fence<T: Resource + Clone>(
    res: Option<Res<T>>,
    mut tracker: ResMut<ResourceTracker<T>>,
    mut inner: NonSendMut<HotReloadInnerApp>,
) {
    let Some(inner) = &mut inner.app else {
        return;
    };
    let Some(res) = res else {
        return;
    };
    if res.is_changed() {
        let world_tick = inner.world.change_tick();
        if world_tick.is_newer_than(tracker.0, world_tick) {
            inner.world.insert_resource(res.as_ref().clone());
            tracker.0 = world_tick;
        }
    }
}

fn sync_from_app<T: Resource + Clone>(
    mut commands: Commands,
    mut tracker: ResMut<ResourceTracker<T>>,
    inner: NonSend<HotReloadInnerApp>,
) {
    let Some(inner) = inner.app.as_ref() else {
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
        commands.insert_resource(r.clone());
        tracker.0 = *world_tick;
    }
}
