use dexterous_developer_instance::internal::HOT_RELOAD_INFO;
use dioxus::prelude::*;
use crate::ReloadableDioxusApp;

pub fn use_background_hotreloader<App: ReloadableDioxusApp>(app: App) -> Element {
    let mut update = use_signal(|| 0u32);
    let info = HOT_RELOAD_INFO
        .get()
        .expect("Hot Reload Info hasn't been set");

    info.update();
    

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if info.update_ready() {
                info.update();
                update += 1;
            }
        }
    });

    update.with(|_| {
        info.call_return(App::name(), &mut ()).unwrap()
    })
}