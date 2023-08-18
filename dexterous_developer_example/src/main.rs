use dexterous_developer::hot_bevy_loader;

fn main() {
    println!("Main Thread: {:?}", std::thread::current().id());
    hot_bevy_loader!(
        lib_dexterous_developer_example::bevy_main,
        dexterous_developer::HotReloadOptions::default()
    );
}
