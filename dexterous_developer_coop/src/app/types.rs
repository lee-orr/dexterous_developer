use std::{
    marker::PhantomData,
    sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use bevy::prelude::{App, Event, Resource};

use crate::{
    coop::types::Coop,
    types::{CoopCommunicationManagerInApp, CoopValue},
};

pub trait SetupCoopCommunication {
    fn setup_incoming_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self);

    fn setup_outgoing_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self);

    fn setup_read_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self);

    fn setup_write_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self);
}

#[derive(Resource)]
pub enum CoopCommunicationManager {
    #[cfg(feature = "direct_memory")]
    DirectMemory(super::direct_memory::DirectMemoryCommunicationManager),
}

impl CoopCommunicationManagerInApp for CoopCommunicationManager {
    fn setup_incoming_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App) {
        todo!()
    }

    fn setup_outgoing_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App) {
        todo!()
    }

    fn setup_read_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App) {
        todo!()
    }

    fn setup_write_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App) {
        todo!()
    }
}

#[derive(Clone, Event)]
pub struct IncomingEvent<T: CoopValue<S>, S: rkyv::Fallible>(T, PhantomData<S>);

impl<T: CoopValue<S>, S: rkyv::Fallible> IncomingEvent<T, S> {
    pub(crate) fn new(event: T) -> Self {
        Self(event, Default::default())
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

#[derive(Clone, Event)]
pub struct OutgoingEvent<T: CoopValue<S>, S: rkyv::Fallible>(T, PhantomData<S>);

impl<T: CoopValue<S>, S: rkyv::Fallible> OutgoingEvent<T, S> {
    pub fn new(event: T) -> Self {
        Self(event, Default::default())
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

#[derive(Resource, Default)]
pub struct ReadBlackboard<T: CoopValue<S>, S: rkyv::Fallible>(Option<Arc<T>>, PhantomData<S>);

impl<T: CoopValue<S>, S: rkyv::Fallible> ReadBlackboard<T, S> {
    pub fn get(&self) -> Option<&T> {
        return self.0.as_ref().map(|v| v.as_ref());
    }

    pub(crate) fn set(&mut self, val: Option<T>) {
        self.0 = val.map(|v| Arc::new(v));
    }
}

#[derive(Resource, Default)]
pub struct WriteBlackboard<T: CoopValue<S>, S: rkyv::Fallible>(
    Option<Arc<RwLock<T>>>,
    PhantomData<S>,
);

impl<T: CoopValue<S>, S: rkyv::Fallible> WriteBlackboard<T, S> {
    pub fn get<'a>(&'a self) -> Option<RwLockReadGuard<'a, T>> {
        self.0.as_ref().and_then(|v| v.read().ok())
    }

    pub fn get_mut(&mut self) -> Option<RwLockWriteGuard<'_, T>> {
        self.0.as_mut().and_then(|v| v.write().ok())
    }

    pub fn set(&mut self, val: Option<T>) {
        self.0 = val.map(|v| Arc::new(RwLock::new(v)));
    }
}
