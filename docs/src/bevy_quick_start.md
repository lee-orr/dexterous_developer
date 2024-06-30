# Bevy Quick Start

## Installation

Install the CLI by running: ```cargo install dexterous_developer_cli```. This installs 2 command line utilities:

- `dexterous_developer_cli` - used to build the project
- `dexterous_developer_runner` - used to run the project

## General Setup

For dexterous_developer to function, your package currently needs to be a dynamic library. To do so, you will need to mark it as a library and add the "dylib" crate type to it in your `Cargo.toml` - ideally in addition to `rlib`. You'll need to add a separate binary for the non-hot reloaded version.

```toml
[lib]
crate-type = ["rlib", "dylib"]
```

You'll also need to add the appropriate dexterous developer adapter to your library's dependencies, and set up the "hot" feature. For example, if you are using bevy:

```toml

[features]
hot = ["dexterous_developer/hot"]

[dependencies]
bevy = "0.14"
dexterous_developer = { version = "0.3", features = ["bevy"] }
serde = "1" # If you want the serialization capacities
```

Finally, you'll need to set up a `Dexterous.toml` file`

```toml
features = [
    "hot"
]
code_watch_folders = ["./src"]
asset_folders = ["./assets"]
```

## Bevy Code

Replace `main.rs` with `lib.rs`, and wrap your main function with the `reloadable_main` macro:

```rust
reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>()) // You can use either DefaultPlugins or MinimnalPlugins here, and use "set" on this as you would with them
    // Here you can do what you'd normally do with app
    // ... and so on
});
```

And wherever you want to add reloadable elements - such as systems, components, or resources - to your app, do so within a `reloadable_scope!` macro - and add the reloadable scope to the app using `setup_reloadable_elements`:

```rust

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app
            .setup_reloadable_elements::<reloadable>();
    }
}

reloadable_scope!(reloadable(app) {
    app
        .add_systems(Update, this_system_will_reload);
})
```

## Running with Hot Reload

To run a hot-reloaded app, you need to do 2 things:

- run the `dexterous_developer_cli` command to set up a build server
- run the `dexterous_developer_runner` command, ideally in a dedicated directory, to actually run the application

If you want to run the application on another machine *with the same platform*, you can run `dexterous_developer_runner` there instead, and pass in a network address to the machine with the build server.
