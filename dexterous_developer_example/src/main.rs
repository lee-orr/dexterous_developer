use dexterous_developer::HotReloadOptions;

fn main() {
    println!("Main Thread: {:?}", std::thread::current().id());
    lib_dexterous_developer_example::bevy_main(HotReloadOptions::default());
}
