# Dexterous Developer
![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/lee-orr/dexterous_developer/.github%2Fworkflows%2Fci.yml?label=CI)
 ![GitHub Workflow Status (with event)](https://img.shields.io/github/actions/workflow/status/lee-orr/dexterous_developer/.github%2Fworkflows%2Fpublish_docs.yaml?label=Publish%20Docs)
 ![crates.io](https://img.shields.io/crates/v/dexterous_developer?label=dexterous_developer) ![cli](https://img.shields.io/crates/v/dexterous_developer_cli?label=dexterous_developer_cli)
![Static Badge](https://img.shields.io/badge/docs-github_pages-green?link=https%3A%2F%2Flee-orr.github.io%2Fdexterous_developer%2F)


An experimental hot reload system for the bevy game engine. Inspired by [DGriffin91's Ridiculous bevy hot reloading](https://github.com/DGriffin91/ridiculous_bevy_hot_reloading) - adding the ability to re-load arbitrary systems, and the ability to transform resource/component structures over time.

Fuller documentation is available at: <https://lee-orr.github.io/dexterous_developer/>

## Features

- Define the reloadable areas of your game explicitly - which can include systems, components and resources (w/ some limitations)
- Reset resources to a default or pre-determined value upon reload
- serialize/deserialize your reloadable resources & components, allowing you to evolve their schemas so long as they are compatible with the de-serializer (using rmp_serde)
- mark entities to get removed on hot reload
- run systems after hot-reload
- create functions to set-up & tear down upon either entering/exiting a state or on hot reload
- only includes any hot reload capacity in your build when you explicitly enable it - such as by using the CLI launcher

## Known issues

- Won't work on mobile or WASM, and only tested on Windows
- events and states still need to be pre-defined
- can't guarantee the main bevy thread will run on the main thread from an OS perspective, which can cause issues in some packages
  - Relies on a fork of bevy_winit (only when hot reload is enabled, will use the regular bevy_winit otherwise). A PR for adding the required ability to bevy exists.

## Installation

Grab the CLI by running: ```cargo install dexterous_developer_cli```. If you don't want to use the cli - there is more info in the docs.

You'll be able to run the dexterous verion of your code by running `dexterous_developer_cli` in your terminal.

In your `Cargo.toml` add the following:

```toml
[lib]
name = "lib_THE_NAME_OF_YOUR_GAME"
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[dependencies]
bevy = "0.11"
dexterous_developer = "0.0.3"
serde = "1" # If you want the serialization capacities
```

If your game is not a library yet, move all your main logic to `lib.rs` rather than `main.rs`. Then, in your `main.rs` - call the bevy_main function:

```rust
fn main() {
    lib_NAME_OF_YOUR_GAME::bevy_main();
}

```

and in your `lib.rs`, your main function should become:

```rust
#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>()) // You can use either DefaultPlugins or MinimnalPlugins here, and use "set" on this as you would with them
    // Here you can do what you'd normally do with app
    // ... and so on
}
```

If you have a plugin where you want to add reloadable elements, add the following in the file defining the plugin:

```rust

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app
            .setup_reloadable_elements::<reloadable>();
    }
}

#[dexterous_developer_setup]
fn reloadable(app: &mut ReloadableAppContents) {
    app
        .add_systems(Update, this_system_will_reload);
}

```

For the best results, you'll want to add the following to your `.cargo/config.toml`:

```toml
# Add the contents of this file to `config.toml` to enable "fast build" configuration. Please read the notes below.

# NOTE: For maximum performance, build using a nightly compiler
# If you are using rust stable, remove the "-Zshare-generics=y" below.

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-Clink-arg=-fuse-ld=lld", "-Zshare-generics=y"]

# NOTE: you must install [Mach-O LLD Port](https://lld.llvm.org/MachO/index.html) on mac. you can easily do this by installing llvm which includes lld with the "brew" package manager:
# `brew install llvm`
[target.x86_64-apple-darwin]
rustflags = [
    "-C",
    "link-arg=-fuse-ld=/usr/local/opt/llvm/bin/ld64.lld",
    "-Zshare-generics=y",
]

[target.aarch64-apple-darwin]
rustflags = [
    "-C",
    "link-arg=-fuse-ld=/opt/homebrew/opt/llvm/bin/ld64.lld",
    "-Zshare-generics=y",
]

[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"
rustflags = ["-Zshare-generics=n"]

# Optional: Uncommenting the following improves compile times, but reduces the amount of debug info to 'line number tables only'
# In most cases the gains are negligible, but if you are on macos and have slow compile times you should see significant gains.
#[profile.dev]
#debug = 1

# Enable a small amount of optimization in debug mode
[profile.dev]
opt-level = 1

# Enable high optimizations for dependencies (incl. Bevy), but not for our code:
[profile.dev.package."*"]
opt-level = 3

[profile.dev.package.gfx-backend-vulkan]
opt-level = 3
debug-assertions = false

```
