use hot_reload::HotReloadOptions;

fn main() {
    println!("Main Thread: {:?}", std::thread::current().id());
    lib_hot_reload_example::bevy_main(HotReloadOptions::default());
}
