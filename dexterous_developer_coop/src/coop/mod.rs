use crate::app::CoopedApp;

pub trait Coop {
    fn set_running_app(&mut self, app: impl CoopedApp);

    fn pause_app(&mut self);

    fn clear_app(&mut self);
}
