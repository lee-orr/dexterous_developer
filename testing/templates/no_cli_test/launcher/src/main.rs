use dexterous_developer::HotReloadOptions;
use log::info;

fn main() {
    env_logger::init();
    info!("Env logger working...");
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("simple".to_string()),
        ..Default::default()
    })
    .expect("Run Reloadable App Failed")
}
