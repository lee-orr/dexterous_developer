use std::marker::PhantomData;

use bevy::{ecs::component::Tick, prelude::*, utils::HashMap};

use crate::FenceAppSync;

use super::{
    schedules::{SyncFromApp, SyncFromFence},
    HotReloadInnerApp,
};

pub struct ComponentSync<T: Component + FenceAppSync<M>, M: Send + Sync + 'static>(
    bool,
    bool,
    PhantomData<(T, M)>,
);

impl<T: Component + FenceAppSync<M>, M: Send + Sync + 'static> ComponentSync<T, M> {
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

impl<T: Component + FenceAppSync<M>, M: Send + Sync + 'static> Plugin for ComponentSync<T, M> {
    fn build(&self, app: &mut App) {
        app.init_resource::<EntityMapper>();
        if self.0 {
            app.add_systems(SyncFromFence, sync_from_fence::<T, M>);
        }
        if self.1 {
            app.add_systems(SyncFromApp, sync_from_app::<T, M>);
        }
    }
}

#[derive(Resource, Default)]
struct EntityMapper(HashMap<Entity, Entity>, HashMap<Entity, Entity>);

#[derive(Component)]
struct ComponentChangeTracker<T: Component>(Tick, PhantomData<T>);

fn sync_from_fence<T: Component + FenceAppSync<M>, M: Send + Sync + 'static>(
    query: Query<(Entity, &T, Option<&ComponentChangeTracker<T>>), Changed<T>>,
    mut mapper: ResMut<EntityMapper>,
    mut inner: NonSendMut<HotReloadInnerApp>,
    mut commands: Commands,
) {
    let Some(inner) = inner.get_app_mut() else {
        return;
    };

    let world_tick = inner.world.change_tick();

    let world = &mut inner.world;

    for (entity, component, tracker) in &query {
        if let Some(tracker) = tracker {
            if !world_tick.is_newer_than(tracker.0, world_tick) {
                continue;
            }
        }
        commands
            .entity(entity)
            .insert(ComponentChangeTracker(world_tick, PhantomData::<T>));
        if let Some(target) = mapper.0.get(&entity) {
            if let Some(mut target) = world.get_entity_mut(*target) {
                target.insert(component.sync_from_fence());
                continue;
            }
            let target = *target;
            let _ = mapper.0.remove(&entity);
            let _ = mapper.1.remove(&target);
        }
        let target = world.spawn(component.sync_from_fence()).id();
        mapper.0.insert(entity, target);
        mapper.1.insert(target, entity);
    }
}

fn sync_from_app<T: Component + FenceAppSync<M>, M: Send + Sync + 'static>(
    mut mapper: ResMut<EntityMapper>,
    mut inner: NonSendMut<HotReloadInnerApp>,
    mut commands: Commands,
) {
    let Some(inner) = inner.get_app_mut() else {
        return;
    };

    let world_tick = inner.world.change_tick();

    let world = &mut inner.world;

    let mut changed = world.query_filtered::<(Entity, &T), Changed<T>>();

    for (entity, component) in changed.iter(world) {
        if let Some(target) = mapper.1.get(&entity) {
            if let Some(mut target) = commands.get_entity(*target) {
                target.insert((
                    component.sync_from_fence(),
                    ComponentChangeTracker(world_tick, PhantomData::<T>),
                ));

                continue;
            }
            let target = *target;
            let _ = mapper.1.remove(&entity);
            let _ = mapper.0.remove(&target);
        }
        let target = commands
            .spawn((
                ComponentChangeTracker(world_tick, PhantomData::<T>),
                component.sync_from_fence(),
            ))
            .id();
        mapper.1.insert(entity, target);
        mapper.0.insert(target, entity);
    }
}
