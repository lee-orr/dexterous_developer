use std::marker::PhantomData;

use bevy::prelude::App;

use crate::types::{sealed, Coop, CoopCommunicationProtocol, CoopedApp};

pub struct DefaultCoop {
    app: Option<Box<dyn CoopedApp>>,
    is_running: bool,
}

impl DefaultCoop {
    fn new() -> Self {
        Self {
            app: None,
            is_running: true,
        }
    }
}

impl sealed::CoopCommunicationProtocol for DefaultCoop {}

impl Coop for DefaultCoop {
    fn set_running_app(&mut self, app: impl CoopedApp) {
        self.app = Some(Box::new(app));
    }

    fn pause_app(&mut self) {
        self.is_running = false;
    }

    fn clear_app(&mut self) {
        self.app = None;
    }
}

impl CoopCommunicationProtocol for DefaultCoop {
    fn get_outgoing_event_bus<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> impl crate::types::CoopMessageBusSender<T, S> {
        todo!()
    }

    fn get_incoming_event_bus<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> impl crate::types::CoopMessageBusSender<T, S> {
        todo!()
    }

    fn get_shared_blackboard_writer<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> impl crate::types::CoopSharedBlackboardWriter<T, S> {
        todo!()
    }

    fn get_shared_blackboard_reader<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> impl crate::types::CoopSharedBlackboardReader<T, S> {
        todo!()
    }

    fn get_bidirectional_shared_blackboard<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> impl crate::types::CoopSharedBlackboardWriter<T, S> {
        todo!()
    }
}
