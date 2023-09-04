# Installation

Grab the CLI by running: ```cargo install dexterous_developer_cli```.

You'll be able to run the dexterous verion of your code by running `dexterous_developer_cli` in your terminal.

In your `Cargo.toml` add the following:

```toml
[lib]
name = "lib_THE_NAME_OF_YOUR_GAME"
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]

[dependencies]
bevy = "0.11"
dexterous_developer = "0.0.9"
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

You might also want the following in your `.cargo/config.toml`:

```toml
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

Note that the CLI will automatically add some additional build flags for dynamic linking, but you can include them in the config.toml as well:

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
```
