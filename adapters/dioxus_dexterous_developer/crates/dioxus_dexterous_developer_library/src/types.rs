use dioxus::dioxus_core::Element;

pub trait ReloadableDioxusApp {
    fn name() -> &'static str;

    fn call() -> Element;
}