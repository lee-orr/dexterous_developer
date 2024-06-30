use std::num::NonZero;

use bevy::{
    app::{AppExit, Startup, Update},
    prelude::*,
    MinimalPlugins,
};
use bevy_dexterous_developer::*;

fn terminal_runner(mut app: App) -> AppExit {
    app.update();
    eprintln!("Ready for Input");
    for line in std::io::stdin().lines() {
        let typed: String = line.unwrap_or_default();
        if typed == "exit" {
            println!("Exiting");
            return AppExit::Success;
        }
        app.update();
    }
    AppExit::Error(NonZero::<u8>::new(1).unwrap())
}

#[derive(Component, Debug)]
struct MyComponent {
    first_field: String
}

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<reloadable>()
        .run();
});

fn update(res : Query<&MyComponent>, mut commands : Commands) {
    let mut list = res.iter().map(|component| {
        format!("{}", component.first_field)
    }).collect::<Vec<_>>();
    list.sort();
    let value = list.join(" - ");
    println!("{value}");
    commands.spawn(MyComponent { first_field: "b".to_string()});
}

fn startup(mut commands : Commands) {
    println!("Press Enter to Progress, or type 'exit' to exit");
    commands.spawn(MyComponent { first_field: "a".to_string()});
}

reloadable_scope!(reloadable(app) {
    app
        .add_systems(Startup, startup)
        .add_systems(Update, update)
        .reset_setup::<MyComponent, _>(startup);
});
