use dioxus::prelude::*;
use dioxus_dexterous_developer::*;

reloadable_main!(dioxus_main {
    launch(|| use_background_hotreloader(App));
});

reloadable_app!(App {

    let value = "Will it load? 🐱";
    let mut count = use_signal(|| 0);
    let mut names = use_signal(|| vec!["John".to_string(), "Doe".to_string()]);

    dioxus::desktop::window().webview.zoom(2.0);

    //
    rsx! {
        li { background_color: "green", "This is value: {value}" }
        ul {
            for item in 0..count() {
                li { "{item}" }
            }
            br {}
            button {
                onclick: move |_| {
                    count += 1;
                    names.push(" 3 Name".to_string());
                    names.push(" 1 Name".to_string());
                },
                "Icrement"
            }
            button {
                onclick: move |_| {
                    names.push(" 123 Name".to_string());
                    names.clear();
                    count -= 1;
                },
                "Decremenet"
            }
            "Count {count}"
            for name in names.iter() {
                li { "{name}" }
            }
        }
    }
});
