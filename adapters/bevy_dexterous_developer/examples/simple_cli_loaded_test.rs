use bevy::{app::{Startup, Update}, log::info, prelude::App, MinimalPlugins};
use bevy_dexterous_developer::*;


fn terminal_runner(mut app: App) {
    app.update();
    for line in std::io::stdin().lines() {
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            println!("Exiting");
            return;
        }
        info!("Running The Update With Bevy Logs");
        println!("Running Update");
        app.update();
        println!("Update Ended");
    }
}

reloadable_main!( bevy_main(initial_plugins) {
    println!("RUNNING INTERNAL MAIN");
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<reloadable>()
        .run();
});


fn update() {
    println!("Hey");
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

reloadable_scope!(reloadable(app) {
    println!("Setting Up Reloadable Scope");
    app.add_systems(Startup, startup)
        .add_systems(Update, update);
    println!("Setup Scope");
});