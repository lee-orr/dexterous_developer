# Release Notes

## UPCOMING

- add support for hot re-loading states
- add support for hot re-loading events
- add support for running examples with the `dylib` crate type.

## Version 0.0.11

- generate a temporary manifest with dylib - avoiding the need to set that up in advance
- make sure compilation errors are emmitted properly
- Add ReloadSettings struct, which for now allows displaying the last update time in the window title, making it clearer when a reload occured
- Add ability to cycle between reload modes - Full reload, Systems and Setup or Systems Only
- Add ability to limit more advance reload to a single "reloadable element" (i.e. reloadable setup function)

## Version 0.0.10

- Fix handling of assets when using `dexterous_developer_cli run` or `remote`
- Add ability to pass in multiple watch directories instead of just watching `src` on the package being built

## Version 0.0.9

- Add tests for networked hot reload
- Add `run`, `serve`, `remote`, `compile-libs`, `run-existing` and `install-cross` commands to the CLI
- Add ability to compile reloadable libraries and load existing libraries
- Add cross compiled, remote hot reload
- Mold support has changed - the path to mold needs to be provided via the `DEXTEROUS_DEVELOPER_LD_PATH` environment variable

## Version 0.0.8

- Add automated tests to validate hot reload across mac, windows & linux.
- Removed the `cargo` crate in favour of using the `cargo-metadata` crate and commands.
- Added checks for clang, lld, and the like.
- Ensure automated tests don't hang infinitely.
- Ensure tests only build the CLI once.
- Split up CI to run tests separately from clippy & doc validation.
- Re-organize git repo so the tests & examples are not part of the root cargo workspace.
- Move all bevy-related elements to their own module in dexterous_developer
- Use ffi-safe &CStr to transfer the initial paths from the "launcher" to the app - avoiding attempts at allocating massive strings for those paths due to rust ABI incompatibilities.
- Output hot reloaded libs to a "hot" subfolder in the target, allowing us to avoid issues with cargo attempting to write over files that are in use.
- Don't replace hashed lib files - like bevy_dylib - when hot reloading.
- Use a sub-process to re-launch the CLI with dynamic linking set up properly, and pre-create the required folders, since on Unix dynamic link search folders are set when the application starts and are not re-evaluated if the environment changes.
