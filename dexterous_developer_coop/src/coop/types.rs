use std::sync::Arc;

use thiserror::Error;

use crate::types::{CoopCommunicationProtocol, CoopValue};

pub trait Coop: CoopCommunicationProtocol {
    fn set_running_app(&mut self, app: impl CoopedApp + 'static);

    fn pause_app(&mut self);

    fn clear_app(&mut self);
}
pub trait CoopedApp {
    fn build<T: CoopCommunicationProtocol>(&mut self, comms: T)
    where
        Self: Sized;

    fn run_frame(&mut self);
}
