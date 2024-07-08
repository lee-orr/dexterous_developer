use dioxus::prelude::*;
use crate::ReloadableDioxusApp;

pub fn use_background_hotreloader<App: ReloadableDioxusApp>() -> Element {
    App::call()
}