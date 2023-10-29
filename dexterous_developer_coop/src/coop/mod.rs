use crate::app::CoopedApp;

pub trait Coop {
    fn set_running_app(&mut self, app: impl CoopedApp);

    fn pause_app(&mut self);

    fn clear_app(&mut self);

    fn send_event<T: rkyv::Serialize<S> + rkyv::Deserialize<T, S>, S: rkyv::Fallible>(&mut self);

    fn register_shared_blackboard<
        T: rkyv::Serialize<S> + rkyv::Deserialize<T, S>,
        S: rkyv::Fallible,
    >(
        &mut self,
    );
}
