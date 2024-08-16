# Bevy Quick Start

## Installation

Install the CLI by running: ```cargo install dexterous_developer_cli```. This installs 2 command line utilities:

- `dexterous_developer_cli` - used to build the project, potentially running it at the same time
- `dexterous_developer_runner` - used to run the project on another device

### Note for pre-relesse versions

Make sure that the version of dexterous_developer_cli matches the version you are installing. While the goal is to eventually have more separation between the two, for now they should be kept in sync.

## General Setup

> **Dynamic Libraries**
>
> Previous versions of Dexterous Developer required you to make your crate a dynamic library. That is no longer necessary!
>
> You can still use an explicit dynamic library as your main crate just like before, but you can also place the `reloadable_main!` macro in `main.rs`, and not provide it with a function name - it will default to "main":
>
> ```rust
> reloadable_main!((initial_plugins) {
>   App::new()
>   ...
> })
> ```

You'll also need to add the appropriate dexterous developer adapter to your library's dependencies, and set up the "hot" feature. For example, if you are using bevy:

```toml

[features]
hot = ["dexterous_developer/hot"]

[dependencies]
bevy = "0.14"
dexterous_developer = { version = "0.4.0-pre.0", features = ["bevy"] }
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

In `main.rs`, wrap your main function with the `reloadable_main` macro:

```rust
reloadable_main!((initial_plugins) {
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

You can then run the non-hot-reloadable version of your app using `cargo run --bin launcher` (or build with `cargo build --bin launcher`). Remember to avoid including the `hot` feature, since it's designed to work only inside a reloadable library!.
