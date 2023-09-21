# Running Without the CLI

To run without the CLI, you need to create a new launcher crate within the same workspace. The recommended approach is to use cargo workspaces to do so. Within that crate, you only really need one dependency:

```toml
[dependencies]
dexterous_developer = { version = "0.0.10", default-features = false, features = [
    "hot",
    "cli",
] }
```

In addition, you'll need to add `dylib` to the crate type for your main game.

```toml
[lib]
name = "lib_THE_NAME_OF_YOUR_GAME"
path = "src/lib.rs"
crate-type = ["rlib", "dylib"]
```

Then in the `main.rs` file, you'd want to trigger the launcher using `run_reloadable_app` - like so:

```rust
use dexterous_developer::HotReloadOptions;

fn main() {
    dexterous_developer::run_reloadabe_app(HotReloadOptions {
        package: Some("NAME_OF_YOUR_GAME_PACKAGE".to_string()),
        ..Default::default()
    })
}
```

The HotReloadOptions can also contain things like features, a custom library name, the watch folder, and the target folder - but it should infer most of that from the package.

You would then run the game using `cargo run -p NAME_OF_THE_LAUNCHER`.
