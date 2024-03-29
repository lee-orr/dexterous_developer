# Dexterous Developer

![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/lee-orr/dexterous_developer/.github%2Fworkflows%2Fci.yml?label=CI)
 ![crates.io](https://img.shields.io/crates/v/dexterous_developer?label=dexterous_developer) ![cli](https://img.shields.io/crates/v/dexterous_developer_cli?label=dexterous_developer_cli)
![Static Badge](https://img.shields.io/badge/docs-github_pages-green?link=https%3A%2F%2Flee-orr.github.io%2Fdexterous_developer%2F)

An experimental hot reload system for the bevy game engine. Inspired by [DGriffin91's Ridiculous bevy hot reloading](https://github.com/DGriffin91/ridiculous_bevy_hot_reloading) - adding the ability to re-load arbitrary systems, and the ability to transform resource/component structures over time.

Fuller documentation is available at: <https://lee-orr.github.io/dexterous_developer/>

## Features

- Define the reloadable areas of your game explicitly - which can include systems, components, states, events and resources (w/ some limitations)
- Reset resources to a default or pre-determined value upon reload
- Serialize/deserialize your reloadable resources & components, allowing you to evolve their schemas so long as they are compatible with the de-serializer (using rmp_serde)
- Mark entities to get removed on hot reload
- Run systems after hot-reload
- Create functions to set up & tear down upon either entering/exiting a state or on hot reload
- Only includes any hot reload capacity in your build when you explicitly enable it - such as by using the CLI launcher
- ~~Cross-platform/cross-device hot reload - run a "hot reload server" on a development environment, and execute the application elsewhere. For best results, the dev environment should be a linux device or a Linux-based development container, but it can be configured to work directly on windows or mac as well - albeit less reliably. Support for cross-compilation directly on windows/mac is not a priority, since those can always be set up to host a docker-in-docker environment with linux, which is confirmed to work.~~ **This feature does not work in the current version, but will be re-implemented in the future**

## Known issues

- Won't work on mobile or WASM

## Installation

Grab the CLI by running: ```cargo install dexterous_developer_cli```.

You'll be able to run the dexterous version of your code by running `dexterous_developer_cli run` in your terminal.

In your `Cargo.toml` add the following:

```toml
[lib]
name = "lib_THE_NAME_OF_YOUR_GAME"
path = "src/lib.rs"
crate-type = ["rlib"]

[dependencies]
bevy = "0.13"
dexterous_developer = "0.1.1"
serde = "1" # If you want the serialization capacities

[package.metadata]
hot_reload_features = ["bevy/dynamic_linking", "bevy/embedded_watcher"] # this injects these features into the build, enabling the use of bevy's dynamic linking and asset hot reload capacity.
```

If your game is not a library yet, move all your main logic to `lib.rs` rather than `main.rs`. Then, in your `main.rs` - call the bevy_main function:

```rust
fn main() {
    lib_NAME_OF_YOUR_GAME::bevy_main();
}

```

and in your `lib.rs`, your main function should become:

```rust
reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>()) // You can use either DefaultPlugins or MinimnalPlugins here, and use "set" on this as you would with them
    // Here you can do what you'd normally do with app
    // ... and so on
});
```

If you have a plugin where you want to add reloadable elements, add the following in the file defining the plugin:

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
}

```

## Bevy Version Support

| Bevy | Dexterous Developer |
| --- |--------------------|
| 0.13 | >= 0.2 |
| 0.12 | 0.0.12, 0.1        |
| 0.11 | <= 0.0.11          |
