use std::sync::Arc;

use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect, TypePath},
};
use thiserror::Error;
pub trait CoopValue: Clone + Reflect + FromReflect + TypePath + Sized + 'static {}

impl<T: Clone + Reflect + FromReflect + TypePath + Sized> CoopValue for T {}

pub trait CoopPlugin {
    fn name() -> &'static str;

    fn setup_app(app: &mut App);
    fn setup_coop(coop: &mut impl CoopCommunicationProtocol);

    fn instantiate() -> Option<Self>
    where
        Self: std::marker::Sized;
}

pub trait CoopCommunicationManagerInApp {
    fn get_incoming_events<T: CoopValue>(&self) -> Arc<[T]>;

    fn register_incoming_events<T: CoopValue>(&mut self);

    fn send_outgoing_events<T: CoopValue>(&self, value: Arc<[T]>);

    fn register_outgoig_events<T: CoopValue>(&mut self);

    fn get_blackboard_value<T: CoopValue>(&self, blackboard: &mut Option<Arc<T>>);

    fn register_blackboard_read_value<T: CoopValue>(&mut self);

    fn set_blackboard_value<T: CoopValue>(&self, blackboard: Option<&T>);

    fn register_blackboard_write_value<T: CoopValue>(&mut self);

    fn request_coop_plugin<T: CoopPlugin>(&mut self);
}

pub trait CoopCommunicationProtocol {
    fn send_event<T: CoopValue>(&self, value: T) -> Result<(), CoopProtocolError>;

    fn get_app_events<T: CoopValue>(&mut self) -> Result<&[T], CoopProtocolError>;

    fn get_shared_blackboard_writer<T: CoopValue>(&mut self) -> Result<&mut T, CoopProtocolError>;

    fn get_shared_blackboard_reader<T: CoopValue>(&mut self) -> Result<&T, CoopProtocolError>;
}

#[derive(Error, Debug)]
pub enum CoopProtocolError {
    #[error("Couldn't get incoming message bus")]
    CouldntGetIncomingBus,
    #[error("Couldn't get incoming message bus")]
    CouldntGetOutgoingBus,

    #[error("Couldn't get blackboard reader")]
    CouldntGetBlackboardReader,
    #[error("Couldn't get blackboard writer")]
    CouldntGetBlackboardWriter,

    #[error("Couldn't send message")]
    CouldntSendMessage,
    #[error("Couldn't receive message")]
    CouldntReceiveMessage,

    #[error("Couldn't get object from reflect")]
    CouldntGetObjectFromReflect,

    #[error("Couldn't get ref from blackboard")]
    CouldntGetRefFromBlackboard,
}
