use super::types::SetupCoopCommunication;

pub struct DirectMemoryCommunicationManager;

impl SetupCoopCommunication for DirectMemoryCommunicationManager {
    fn setup_incoming_events<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(&mut self) {
        todo!()
    }

    fn setup_outgoing_events<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(&mut self) {
        todo!()
    }

    fn setup_read_blackboard<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(&mut self) {
        todo!()
    }

    fn setup_write_blackboard<T: crate::types::CoopValue<S>, S: rkyv::Fallible>(&mut self) {
        todo!()
    }
}
