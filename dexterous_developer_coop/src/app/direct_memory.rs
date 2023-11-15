use std::sync::Arc;

use bevy::{prelude::App, reflect::Reflect, utils::HashMap};
use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::{
    coop::{
        direct_memory::DirectMemoryCoopHandler,
        types::{CoopedApp, RunningCoopedApp},
    },
    types::CoopCommunicationManagerInApp,
};

pub struct DirectMemoryCommunicationManager {
    incoming_events: HashMap<&'static str, Receiver<Box<dyn Reflect>>>,
    outgoing_events: HashMap<&'static str, Sender<Box<dyn Reflect>>>,
    incoming_blackboard_values: HashMap<&'static str, Receiver<Box<dyn Reflect>>>,
    outgoing_blackboard_values: HashMap<&'static str, Sender<Box<dyn Reflect>>>,
    comms_sender: Sender<InternalMessages>,
}

impl DirectMemoryCommunicationManager {
    pub fn new(comms_sender: Sender<InternalMessages>) -> Self {
        Self {
            incoming_events: Default::default(),
            outgoing_events: Default::default(),
            incoming_blackboard_values: Default::default(),
            outgoing_blackboard_values: Default::default(),
            comms_sender,
        }
    }
}

pub enum InternalMessages {
    RegisterPlugin(&'static str),
    IncomingEvent(&'static str, Sender<Box<dyn Reflect>>),
    OutgoingEvent(&'static str, Receiver<Box<dyn Reflect>>),
    IncomingBlackboard(&'static str, Sender<Box<dyn Reflect>>),
    OutgoingBlackboard(&'static str, Receiver<Box<dyn Reflect>>),
}

impl CoopCommunicationManagerInApp for DirectMemoryCommunicationManager {
    fn get_incoming_events<T: crate::types::CoopValue>(&self) -> std::sync::Arc<[T]> {
        let Some(events) = self.incoming_events.get(T::type_path()) else {
            return Arc::new([]);
        };
        events
            .try_iter()
            .filter_map(|v| v.downcast::<T>().ok())
            .map(|v| v.as_ref().clone())
            .collect()
    }

    fn send_outgoing_events<T: crate::types::CoopValue>(&self, value: std::sync::Arc<[T]>) {
        let Some(events) = self.outgoing_events.get(T::type_path()) else {
            return;
        };
        for event in value.iter() {
            let _ = events.try_send(event.clone_value());
        }
    }

    fn get_blackboard_value<T: crate::types::CoopValue>(&self, blackboard: &mut Option<Arc<T>>) {
        let Some(events) = self.incoming_blackboard_values.get(T::type_path()) else {
            return;
        };

        let Some(value) = events.try_iter().last() else {
            return;
        };

        let Ok(value) = value.downcast::<T>() else {
            return;
        };

        let _ = blackboard.insert(Arc::new(value.as_ref().clone()));
    }

    fn set_blackboard_value<T: crate::types::CoopValue>(&self, blackboard: Option<&T>) {
        let Some(blackboard) = blackboard else {
            return;
        };
        let Some(events) = self.outgoing_blackboard_values.get(T::type_path()) else {
            return;
        };

        let _ = events.try_send(blackboard.clone_value());
    }

    fn request_coop_plugin<T: crate::types::CoopPlugin>(&mut self) {
        let _ = self
            .comms_sender
            .send(InternalMessages::RegisterPlugin(T::name()));
    }

    fn register_incoming_events<T: crate::types::CoopValue>(&mut self) {
        let key = T::type_path();
        if self.incoming_events.contains_key(key) {
            return;
        }

        let (tx, rx) = unbounded::<Box<dyn Reflect>>();

        self.incoming_events.insert(key, rx);
        let _ = self
            .comms_sender
            .send(InternalMessages::IncomingEvent(key, tx));
    }

    fn register_outgoig_events<T: crate::types::CoopValue>(&mut self) {
        let key = T::type_path();
        if self.outgoing_events.contains_key(key) {
            return;
        }

        let (tx, rx) = unbounded::<Box<dyn Reflect>>();

        self.outgoing_events.insert(key, tx);
        let _ = self
            .comms_sender
            .send(InternalMessages::OutgoingEvent(key, rx));
    }

    fn register_blackboard_read_value<T: crate::types::CoopValue>(&mut self) {
        let key = T::type_path();
        if self.incoming_blackboard_values.contains_key(key) {
            return;
        }

        let (tx, rx) = unbounded::<Box<dyn Reflect>>();

        self.incoming_blackboard_values.insert(key, rx);
        let _ = self
            .comms_sender
            .send(InternalMessages::IncomingBlackboard(key, tx));
    }

    fn register_blackboard_write_value<T: crate::types::CoopValue>(&mut self) {
        let key = T::type_path();
        if self.outgoing_blackboard_values.contains_key(key) {
            return;
        }

        let (tx, rx) = unbounded::<Box<dyn Reflect>>();

        self.outgoing_blackboard_values.insert(key, tx);
        let _ = self
            .comms_sender
            .send(InternalMessages::OutgoingBlackboard(key, rx));
    }
}
