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

reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<MinimalPlugins>())
        .set_runner(terminal_runner)
        .setup_reloadable_elements::<reloadable>()
        .run();
});

#[derive(Event, Debug, Clone)]
enum MyEvent {
    A,
    B,
}

fn update(mut reader: EventReader<MyEvent>) -> Option<MyEvent> {
    println!("Running Update");
    let event = reader.read().next().cloned();
    println!("{event:?}");
    event
}
fn publish(previous: In<Option<MyEvent>>, mut writer: EventWriter<MyEvent>) {
    println!("Publish Event");

    match previous.0 {
        Some(MyEvent::A) => writer.send(MyEvent::B),
        Some(MyEvent::B) => writer.send(MyEvent::A),
        None => writer.send(MyEvent::A),
    };
}

fn startup() {
    println!("Press Enter to Progress, or type 'exit' to exit");
}

reloadable_scope!(reloadable(app) {
    app.add_systems(Startup, startup)
        .add_systems(Update, update.pipe(publish))
        .add_event::<MyEvent>();
});
