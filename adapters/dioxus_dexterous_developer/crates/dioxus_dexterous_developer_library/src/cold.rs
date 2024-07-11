use crate::ReloadableDioxusApp;
use dioxus::prelude::*;

pub fn use_background_hotreloader<App: ReloadableDioxusApp>() -> Element {
    App::call()
}
