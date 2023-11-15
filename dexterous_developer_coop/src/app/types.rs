use std::sync::Arc;

use bevy::prelude::{App, Event, EventReader, EventWriter, First, Last, Res, ResMut, Resource};

use crate::types::{CoopCommunicationManagerInApp, CoopPlugin, CoopValue};

pub trait SetupCoopCommunication {
    fn request_coop_plugin<T: CoopPlugin>(&mut self);

    fn setup_incoming_events<T: CoopValue>(&mut self);

    fn setup_outgoing_events<T: CoopValue>(&mut self);

    fn setup_read_blackboard<T: CoopValue>(&mut self);

    fn setup_write_blackboard<T: CoopValue>(&mut self);
}

impl SetupCoopCommunication for App {
    fn request_coop_plugin<T: CoopPlugin>(&mut self) {
        let Some(mut communication_manager) =
            self.world.get_resource_mut::<CoopCommunicationManager>()
        else {
            panic!("No Coop Communication Manager Available")
        };

        communication_manager.request_coop_plugin::<T>();
    }

    fn setup_incoming_events<T: CoopValue>(&mut self) {
        let Some(mut communication_manager) =
            self.world.get_resource_mut::<CoopCommunicationManager>()
        else {
            panic!("No Coop Communication Manager Available")
        };

        communication_manager.register_incoming_events::<T>();

        self.add_event::<IncomingEvent<T>>()
            .add_systems(First, process_incoming_events::<T>);
    }

    fn setup_outgoing_events<T: CoopValue>(&mut self) {
        let Some(mut communication_manager) =
            self.world.get_resource_mut::<CoopCommunicationManager>()
        else {
            panic!("No Coop Communication Manager Available")
        };
        communication_manager.register_outgoig_events::<T>();

        self.add_event::<OutgoingEvent<T>>()
            .add_systems(Last, process_outgoing_events::<T>);
    }

    fn setup_read_blackboard<T: CoopValue>(&mut self) {
        let Some(mut communication_manager) =
            self.world.get_resource_mut::<CoopCommunicationManager>()
        else {
            panic!("No Coop Communication Manager Available")
        };
        communication_manager.register_blackboard_read_value::<T>();

        self.init_resource::<ReadBlackboard<T>>()
            .add_systems(First, process_read_blackboard::<T>);
    }

    fn setup_write_blackboard<T: CoopValue>(&mut self) {
        let Some(mut communication_manager) =
            self.world.get_resource_mut::<CoopCommunicationManager>()
        else {
            panic!("No Coop Communication Manager Available")
        };

        communication_manager.register_blackboard_write_value::<T>();

        self.init_resource::<WriteBlackboard<T>>()
            .add_systems(Last, process_write_blackboard::<T>);
    }
}

#[derive(Resource)]
pub enum CoopCommunicationManager {
    DirectMemory(super::direct_memory::DirectMemoryCommunicationManager),
}

impl CoopCommunicationManagerInApp for CoopCommunicationManager {
    fn get_incoming_events<T: CoopValue>(&self) -> Arc<[T]> {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.get_incoming_events::<T>(),
        }
    }

    fn send_outgoing_events<T: CoopValue>(&self, value: Arc<[T]>) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.send_outgoing_events::<T>(value),
        }
    }

    fn get_blackboard_value<T: CoopValue>(&self, blackboard: &mut Option<Arc<T>>) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.get_blackboard_value::<T>(blackboard),
        }
    }

    fn set_blackboard_value<T: CoopValue>(&self, blackboard: Option<&T>) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.set_blackboard_value::<T>(blackboard),
        }
    }

    fn request_coop_plugin<T: CoopPlugin>(&mut self) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.request_coop_plugin::<T>(),
        }
    }

    fn register_incoming_events<T: CoopValue>(&mut self) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.register_incoming_events::<T>(),
        }
    }

    fn register_outgoig_events<T: CoopValue>(&mut self) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.register_outgoig_events::<T>(),
        }
    }

    fn register_blackboard_read_value<T: CoopValue>(&mut self) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.register_blackboard_read_value::<T>(),
        }
    }

    fn register_blackboard_write_value<T: CoopValue>(&mut self) {
        match self {
            CoopCommunicationManager::DirectMemory(v) => v.register_blackboard_write_value::<T>(),
        }
    }
}

#[derive(Clone, Event)]
pub struct IncomingEvent<T: CoopValue>(T);

impl<T: CoopValue> IncomingEvent<T> {
    pub fn get(&self) -> &T {
        &self.0
    }
}

#[derive(Clone, Event)]
pub struct OutgoingEvent<T: CoopValue>(T);

impl<T: CoopValue> OutgoingEvent<T> {
    pub fn new(event: T) -> Self {
        Self(event)
    }

    pub fn get(&self) -> &T {
        &self.0
    }
}

#[derive(Resource)]
pub struct ReadBlackboard<T: CoopValue>(Option<Arc<T>>);

impl<T: CoopValue> ReadBlackboard<T> {
    pub fn get(&self) -> Option<&T> {
        return self.0.as_ref().map(|v| v.as_ref());
    }
}

impl<T: CoopValue> Default for ReadBlackboard<T> {
    fn default() -> Self {
        Self(None)
    }
}

#[derive(Resource)]
pub struct WriteBlackboard<T: CoopValue>(Option<T>);

impl<T: CoopValue> Default for WriteBlackboard<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: CoopValue> WriteBlackboard<T> {
    pub fn get(&self) -> Option<&T> {
        self.0.as_ref()
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.0.as_mut()
    }

    pub fn set(&mut self, val: Option<T>) {
        self.0 = val;
    }
}

fn process_incoming_events<T: CoopValue>(
    communication_manager: Res<CoopCommunicationManager>,
    mut events: EventWriter<IncomingEvent<T>>,
) {
    let incoming = communication_manager.get_incoming_events::<T>();
    events.send_batch(incoming.iter().map(|v| IncomingEvent(v.clone())));
}

fn process_outgoing_events<T: CoopValue>(
    communication_manager: Res<CoopCommunicationManager>,
    mut events: EventReader<OutgoingEvent<T>>,
) {
    communication_manager.send_outgoing_events(events.read().map(|v| v.0.clone()).collect());
}

fn process_read_blackboard<T: CoopValue>(
    communication_manager: Res<CoopCommunicationManager>,
    mut blackboard: ResMut<ReadBlackboard<T>>,
) {
    let option = &mut blackboard.0;
    communication_manager.get_blackboard_value(option)
}

fn process_write_blackboard<T: CoopValue>(
    communication_manager: Res<CoopCommunicationManager>,
    blackboard: Res<WriteBlackboard<T>>,
) {
    communication_manager.set_blackboard_value(blackboard.0.as_ref());
}
