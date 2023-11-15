use crate::types::CoopCommunicationProtocol;

pub trait Coop: CoopCommunicationProtocol {
    fn set_running_app(&mut self, app: impl CoopedApp + 'static);

    fn pause_app(&mut self);

    fn clear_app(&mut self);

    fn register_plugin_initializers(&mut self);
}
pub trait CoopedApp {
    type InnerApp: RunningCoopedApp;

    fn build(
        &mut self,
        communication_protocol: <Self::InnerApp as RunningCoopedApp>::Comms,
        request_plugins: impl FnMut(&'static str),
    ) -> Self::InnerApp
    where
        Self: Sized;
}

pub trait RunningCoopedApp {
    type Comms: CoopCommunicationProtocol;
    fn run_frame(&mut self);
    fn get_comms(&mut self) -> &mut Self::Comms;
}
