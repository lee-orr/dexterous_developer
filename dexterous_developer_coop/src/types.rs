use std::{marker::PhantomData, sync::Arc};

use self::sealed::CoopSharedBlackboardInner;

pub trait Coop: CoopCommunicationProtocol {
    fn set_running_app(&mut self, app: impl CoopedApp);

    fn pause_app(&mut self);

    fn clear_app(&mut self);
}

pub trait CoopedApp {
    fn build<T: CoopCommunicationProtocol>(&mut self, comms: T)
    where
        Self: Sized;

    fn run_frame(&mut self);
}

pub trait CoopValue<S: rkyv::Fallible>:
    rkyv::Serialize<S> + rkyv::Deserialize<Self, S> + Clone
{
    fn coop_type_name() -> &'static str;
}

#[derive(Clone, bevy::prelude::Event)]
pub struct CoopIncomingEvent<T: CoopValue<S>, S: rkyv::Fallible>(T, PhantomData<S>);

#[derive(Clone, bevy::prelude::Event)]
pub struct CoopOutgoingEvent<T: CoopValue<S>, S: rkyv::Fallible>(T, PhantomData<S>);

pub(crate) mod sealed {
    pub trait CoopSharedBlackboardInner {}

    pub trait CoopMessageBusSender {}

    pub trait CoopMessageBusReceiver {}

    pub trait CoopCommunicationProtocol {}
}

pub trait CoopSharedBlackboardReader<T: CoopValue<S>, S: rkyv::Fallible>:
    sealed::CoopSharedBlackboardInner
{
    fn get(&self) -> &T;
}

pub trait CoopSharedBlackboardWriter<T: CoopValue<S>, S: rkyv::Fallible>:
    CoopSharedBlackboardReader<T, S>
{
    fn get_mut(&mut self) -> &mut T;
    fn set(&mut self, value: T);
}

#[derive(Clone, bevy::prelude::Resource)]
pub struct CoopSharedBlackboardResource<T: CoopValue<S>, S: rkyv::Fallible>(
    Arc<dyn CoopSharedBlackboardReader<T, S>>,
    PhantomData<S>,
);

#[derive(Clone, bevy::prelude::Resource)]
pub struct CoopSharedBlackboardResourceWriter<T: CoopValue<S>, S: rkyv::Fallible>(
    Arc<dyn CoopSharedBlackboardWriter<T, S>>,
    PhantomData<S>,
);

pub trait CoopMessageBusSender<T: CoopValue<S>, S: rkyv::Fallible>:
    sealed::CoopMessageBusSender
{
    fn send_event(&self, value: T);
}

pub trait CoopMessageBusReceiver<T: CoopValue<S>, S: rkyv::Fallible>:
    sealed::CoopMessageBusReceiver
{
    fn recv(&self) -> Option<T>;
}

pub trait CoopCommunicationProtocol {
    fn get_outgoing_event_bus<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> Arc<dyn CoopMessageBusSender<T, S>>;

    fn get_incoming_event_bus<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> Arc<dyn CoopMessageBusSender<T, S>>;

    fn get_shared_blackboard_writer<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> Arc<dyn CoopSharedBlackboardWriter<T, S>>;

    fn get_shared_blackboard_reader<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> Arc<dyn CoopSharedBlackboardReader<T, S>>;

    fn get_bidirectional_shared_blackboard<T: CoopValue<S>, S: rkyv::Fallible>(
        &self,
    ) -> Arc<dyn CoopSharedBlackboardWriter<T, S>>;
}
