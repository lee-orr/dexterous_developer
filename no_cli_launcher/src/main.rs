use dexterous_developer::HotReloadOptions;

fn main() {
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("dexterous_developer_example".to_string()),
        ..Default::default()
    })
}
