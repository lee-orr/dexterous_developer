use dexterous_developer::HotReloadOptions;
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

fn main() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();
    info!("Env logger working...");
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("simple".to_string()),
        ..Default::default()
    })
    .expect("Run Reloadable App Failed")
}
