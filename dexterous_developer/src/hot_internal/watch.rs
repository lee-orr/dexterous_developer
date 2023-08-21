use std::sync::RwLock;

use crate::LibPathSet;

pub struct InitializeWatchClosure(RwLock<Option<fn() -> ()>>);

impl InitializeWatchClosure {
    pub(crate) fn new(func: fn() -> ()) -> Self {
        Self(RwLock::new(Some(func)))
    }

    pub fn run(&self, lib: &LibPathSet, cmd: &str) {
        println!("Attempting to Initialize Watcher");
        if let Ok(read) = self.0.read() {
            if read.is_none() {
                return;
            }
        }
        println!("Initialize Watcher is available");
        let Ok(mut w) = self.0.write() else {
            return;
        };
        println!("Initialize Watcher lock retrieved");
        let Some(call) = w.take() else {
            return;
        };
        println!("Initialize Watcher has been accessed");
        call();
        println!("Initialize Watcher Call Completed");
    }
}

/// SAFETY: InitializeWatchClosure is only initialized when the hot reload plugin is initialized function, and it is static in relation to internally run app.
unsafe impl Sync for InitializeWatchClosure {}

/// SAFETY: InitializeWatchClosure is only initialized when the hot reload plugin is initialized function, and it is static in relation to internally run app.
unsafe impl Send for InitializeWatchClosure {}
