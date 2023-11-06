use std::sync::Arc;

use bevy::{
    prelude::App,
    reflect::{FromReflect, Reflect},
};
use thiserror::Error;
pub trait CoopValue<S: rkyv::Fallible>:
    rkyv::Serialize<S> + rkyv::Deserialize<Self, S> + Clone + Reflect + FromReflect + Sized
{
    fn coop_type_name() -> &'static str;
}

pub trait CoopPlugin {
    fn name() -> &'static str;

    fn setup_app(app: &mut App);
    fn setup_coop(coop: &mut impl CoopCommunicationProtocol);
}

pub trait CoopCommunicationManagerInApp {
    fn setup_incoming_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App);

    fn setup_outgoing_events<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App);

    fn setup_read_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App);

    fn setup_write_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(&mut self, app: &mut App);
}

pub trait CoopCommunicationProtocol {
    fn send_event<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
        value: T,
    ) -> Result<(), CoopProtocolError>;

    fn get_app_events<T: CoopValue<S>, S: rkyv::Fallible>(
        &mut self,
    ) -> Result<Arc<[T]>, CoopProtocolError>;

    fn get_shared_blackboard_writer<T: CoopValue<S>, S: rkyv::Fallible>(
        &mut self,
    ) -> Result<&mut T, CoopProtocolError>;

    fn get_shared_blackboard_reader<T: CoopValue<S>, S: rkyv::Fallible>(
        &mut self,
    ) -> Result<&T, CoopProtocolError>;
}

#[derive(Error, Debug)]
pub enum CoopProtocolError {
    #[error("Couldn't get incoming message bus")]
    CouldntGetIncomingBus(Box<dyn std::error::Error>),
    #[error("Couldn't get incoming message bus")]
    CouldntGetOutgoingBus(Box<dyn std::error::Error>),

    #[error("Couldn't get blackboard reader")]
    CouldntGetBlackboardReader(Box<dyn std::error::Error>),
    #[error("Couldn't get blackboard writer")]
    CouldntGetBlackboardWriter(Box<dyn std::error::Error>),

    #[error("Couldn't send message")]
    CouldntSendMessage(Box<dyn std::error::Error>),
    #[error("Couldn't receive message")]
    CouldntReceiveMessage(Box<dyn std::error::Error>),

    #[error("Couldn't get object from reflect")]
    CouldntGetObjectFromReflect,

    #[error("Couldn't get ref from blackboard")]
    CouldntGetRefFromBlackboard(Box<dyn std::error::Error>),
}
