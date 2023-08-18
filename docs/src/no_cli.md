# Use Without the CLI

If you don't want to use the CLI, you can still develop using dexterous_developer! The main benefit of the CLI is that it avoids compiling dependencies for the launcher, and then compiling them again with different settings when the libraries are loaded. You could achieve the same thing with workspaces in cargo, but for simplicity I'll show the "quick and dirty" option without workspaces.

Most of the steps from the [standard installation](./install.md) are identical - the only change lies in your `main.rs` file. Instead of calling your `bevy_main` function directly, you'll need to use a macro:

```rust
fn main() {
    hot_bevy_loader!(
        lib_NAME_OF_YOUR_GAME::bevy_main,
        dexterous_developer::HotReloadOptions::default()
    );
}

```

The macro recieves a `HotReloadOptions` object, which allows you to provide the following settings:

- `package`: this is the name of the package you are compiling, will default to the name of the executable without the extension
- `lib_name`: this is the name of the library itself - defaults to adding "lib_" to the start of the package name (like the example "lib_NAME_OF_YOUR_GAME")
- `watch_folder`: this is the folder to watch for changes - defaults to the "src" folder in the CARGO_MANIFEST_DIR
- `target_folder`: this is the folder that will contain our library - defaults to the directory containing the current executable (normal "./target/debug")
- `features`: a vec of features you want to enable on the hot reload build. Will always include "bevy/dynamic_linking" and "dexterous_developer/hot_internal", but you need to add any other features here.
