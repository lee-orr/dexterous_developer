use dexterous_developer::HotReloadOptions;

fn main() {
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("simple".to_string()),
        ..Default::default()
    })
}
