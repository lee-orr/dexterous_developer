use bevy::{
    prelude::App,
    reflect::{Array, DynamicArray, Reflect},
    utils::HashMap,
};
use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::{
    app::{
        direct_memory::{DirectMemoryCommunicationManager, InternalMessages},
        types::CoopCommunicationManager,
    },
    types::{CoopCommunicationProtocol, CoopProtocolError},
};

use super::types::{CoopedApp, RunningCoopedApp};

pub struct DirectMemoryCoopHandler {
    incoming_events: HashMap<&'static str, (Receiver<Box<dyn Reflect>>, Box<DynamicArray>)>,
    outgoing_events: HashMap<&'static str, Sender<Box<dyn Reflect>>>,
    incoming_blackboard_values:
        HashMap<&'static str, (Receiver<Box<dyn Reflect>>, Option<Box<dyn Reflect>>)>,
    outgoing_blackboard_values:
        HashMap<&'static str, (Sender<Box<dyn Reflect>>, Option<Box<dyn Reflect>>)>,
}

impl DirectMemoryCoopHandler {
    fn new() -> Self {
        Self {
            incoming_events: Default::default(),
            outgoing_events: Default::default(),
            incoming_blackboard_values: Default::default(),
            outgoing_blackboard_values: Default::default(),
        }
    }

    fn process_internal_messages(mut self,
        comms_receiver: Receiver<InternalMessages>,
        mut request_plugins: impl FnMut(&'static str),
    ) -> Self {
        for msg in comms_receiver.try_iter() {
            match msg {
                InternalMessages::RegisterPlugin(p) => request_plugins(p),
                InternalMessages::IncomingEvent(n, t) => {
                    self.outgoing_events.insert(n, t);
                }
                InternalMessages::OutgoingEvent(n, t) => {
                    self
                        .incoming_events
                        .insert(n, (t, Box::new(DynamicArray::new(Box::new([])))));
                }
                InternalMessages::IncomingBlackboard(n, t) => {
                    self.outgoing_blackboard_values.insert(n, (t, None));
                }
                InternalMessages::OutgoingBlackboard(n, t) => {
                    self.incoming_blackboard_values.insert(n, (t, None));
                }
            }
        }

        self
    }

    fn apply_post_frame_events(&mut self) {
        for (_, (rx, array)) in self.incoming_events.iter_mut() {
            let new_array: Box<[_]> = rx.try_iter().collect();
            let new_array = DynamicArray::new(new_array);
            let new_array = Box::new(new_array);
            *array = new_array;
        }
        for (_, (rx, value)) in self.incoming_blackboard_values.iter_mut() {
            let new_value = rx.try_iter().last();
            *value = new_value;
        }
    }

    fn apply_pre_frame_events(&mut self) {
        for (_, (rx, array)) in self.incoming_events.iter_mut() {
            let new_array: Box<[_]> = rx.try_iter().collect();
            let new_array = DynamicArray::new(new_array);
            let new_array = Box::new(new_array);
            *array = new_array;
        }
        for (_, (rx, value)) in self.incoming_blackboard_values.iter_mut() {
            let new_value = rx.try_iter().last();
            *value = new_value;
        }
    }
}

impl CoopCommunicationProtocol for DirectMemoryCoopHandler {
    fn send_event<T: crate::types::CoopValue>(
        &self,
        value: T,
    ) -> Result<(), crate::types::CoopProtocolError> {
        let Some(events) = self.outgoing_events.get(T::type_path()) else {
            return Err(CoopProtocolError::CouldntGetOutgoingBus);
        };

        let Ok(_) = events.send(Box::new(value)) else {
            return Err(CoopProtocolError::CouldntSendMessage);
        };
        Ok(())
    }

    fn get_app_events<T: crate::types::CoopValue>(
        &mut self,
    ) -> Result<&[T], crate::types::CoopProtocolError> {
        todo!()
    }

    fn get_shared_blackboard_writer<T: crate::types::CoopValue>(
        &mut self,
    ) -> Result<&mut T, crate::types::CoopProtocolError> {
        todo!()
    }

    fn get_shared_blackboard_reader<T: crate::types::CoopValue>(
        &mut self,
    ) -> Result<&T, crate::types::CoopProtocolError> {
        todo!()
    }
}

pub struct DirectMemoryAppRunner {
    app: bevy::prelude::App,
    comms: DirectMemoryCoopHandler,
}

impl RunningCoopedApp for DirectMemoryAppRunner {
    type Comms = DirectMemoryCoopHandler;
    fn run_frame(&mut self) {
        self.app.update();
        self.comms.apply_post_frame_events();
    }

    fn get_comms(&mut self) -> &mut Self::Comms {
        &mut self.comms
    }
}

impl<F: Fn(&mut App)> CoopedApp for F {
    type InnerApp = DirectMemoryAppRunner;

    fn build(
        &mut self,
        mut communication_protocol: <Self::InnerApp as RunningCoopedApp>::Comms,
        request_plugins: impl FnMut(&'static str),
    ) -> Self::InnerApp
    where
        Self: Sized,
    {
        let (tx, rx) = unbounded();
        let mut app = App::new();
        let app_comms: DirectMemoryCommunicationManager = DirectMemoryCommunicationManager::new(tx);
        app.insert_resource(CoopCommunicationManager::DirectMemory(app_comms));
        self(&mut app);

        let comms = DirectMemoryCoopHandler::new().process_internal_messages(rx, request_plugins);
        Self::InnerApp { app, comms }
    }
}
