# Dexterous Developer

![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/lee-orr/dexterous_developer/.github%2Fworkflows%2Ftests.yml?label=Tests)
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/lee-orr/dexterous_developer/.github%2Fworkflows%2Fstatic-analysis.yml?label=Static%20Analysis)
 ![crates.io](https://img.shields.io/crates/v/dexterous_developer?label=dexterous_developer) ![cli](https://img.shields.io/crates/v/dexterous_developer_cli?label=dexterous_developer_cli)

> [!CAUTION]
> This readme is currently in flux, as part of the changes toward version `0.3`. To see the readme for `0.2`, go [here](https://github.com/lee-orr/dexterous_developer/tree/dexterous_developer-v0.2.0)

A modular hot-reload system for Rust.

Docs for the latest release are available at: <https://lee-orr.github.io/dexterous_developer/>

## Features

- A CLI for building & running reloadable rust projects, including over the network (cross-device)
- The ability to serialize/deserialize elements, allowing the evolution of schemas over time
- Only includes any hot reload capacity in your build when you explicitly enable it - such as by using the CLI launcher
- The capacity to create adapters for additional frameworks, allowing you to use Dexterous Developer tooling with other tools.
- Includes a first-party Bevy adapter
- Works on Windows, Linux, and MacOS
- On Linux, can be used to develop within a dev container while running on the main OS, enabling use of dev containers for games & other GUI apps.

### Bevy Specific

- Define the reloadable areas of your game explicitly - which can include systems, components, events and resources (w/ some limitations)
- Reset resources to a default or pre-determined value upon reload
- Serialize/deserialize your reloadable resources & components, allowing you to evolve their schemas so long as they are compatible with the de-serializer (using rmp_serde)
- Mark entities to get removed on hot reload
- Run systems after hot-reload
- Create functions to set up & tear down upon either entering/exiting a state or on hot reload

### Future Work

- Cross-platform hot reload - run a "hot reload server" on a development environment, and execute the application on a different OS
- Mobile support
- Browser-based WASM support
- WASI support
- Patching running libraries with intermediate compilation results
- Supporting the use of inter-process communication in addition to the current dynamic-library approach
- Simplify CLI / Launchers
  - When running in place, avoid needing to copy/download files if running from the same dir
  - GUI-based launchers should be added, especially for mobile
- Supporting state in bevy

## Installation

Install the CLI by running: ```cargo install dexterous_developer_cli```. This installs 2 command line utilities:

- `dexterous_developer_cli` - used to build the project
- `dexterous_developer_runner` - used to run the project

## Setup

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

Finally, you'll need to set up a `Dexterous.toml` file, that helps define some of the necessary elements - such as which folders should be watched for changes, and what features should be enabled. See the [example file in this repository](./Dexterous.toml) or the [book](https://lee-orr.github.io/dexterous_developer/) for more info.

### Bevy Setup

If your game is not a library yet, move all your main logic to `lib.rs` rather than `main.rs`. In your `lib.rs`, your main function should become:

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
})
```

The [Simple Visual](./adapters/bevy_dexterous_developer/examples/simple_visual.rs) example shows the basic use of the library, and the [book](https://lee-orr.github.io/dexterous_developer/) has more info as well.

## Running with Hot Reload

To run a hot-reloaded app locally, cargo install and run `dexterous_developer_cli` (optionally passing in a specific package or example).

To run the app on a different machine (with the same platform), cargo install `dexterous_developer_cli` on both machines, and then:

- run the `dexterous_developer_cli --serve-only` on the development machine
- run the `dexterous_developer_runner --server http://*.*.*.*:4321` command, ideally in a dedicated directory, on the target machine

## Running or Building Without Hot Reload

Once you have everything set up for development, you will likely want to be able to build production versions of the application as well. This will require creating a separate binary. To do so, you can add a `bins/launcher.rs` to your project:

```rust
fn main() {
    PACKAGE_NAME::bevy_main();
}
```

and in your `Cargo.toml`, you'll need to add:

```toml
[[bin]]
name = "launcher"
path = "bins/launcher.rs"
```

You can then run the non-hot-reloadable version of your app using `cargo run --bin launcher`. Remember to avoid including the `hot` feature, since it's designed to work only inside a reloadable library!.

## Inspiration

Initial inspiration came from [DGriffin91's Ridiculous bevy hot reloading](https://github.com/DGriffin91/ridiculous_bevy_hot_reloading)

## Bevy Version Support

| Bevy | Dexterous Developer |
| --- |--------------------|
| 0.14 | >= 0.3 |
| 0.13 | = 0.2 |
| 0.12 | 0.0.12, 0.1        |
| 0.11 | <= 0.0.11          |
