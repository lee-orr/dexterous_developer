use dexterous_developer::HotReloadOptions;

fn main() {
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("tmp_lib_no_cli".to_string()),
        ..Default::default()
    })
}
