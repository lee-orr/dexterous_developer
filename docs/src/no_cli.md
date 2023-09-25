# Running Without the CLI

To run without the CLI, you need to create a new launcher crate within the same workspace. The recommended approach is to use cargo workspaces to do so. Within that crate, you only really need one dependency:

```toml
[dependencies]
dexterous_developer = { version = "0.0.10", default-features = false, features = [
    "hot",
    "cli",
] }
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

You will also need to add `dylib` to the crate type of your main library, or create a separate dynamic library crate that will be loaded by the launcher. When using the CLI it creates a temporary manifest file that adds the dylib - so it's not needed with the CLI.

The HotReloadOptions can also contain things like features, a custom library name, the watch folder, and the target folder - but it should infer most of that from the package.

You would then run the game using `cargo run -p NAME_OF_THE_LAUNCHER`.
