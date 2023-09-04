# Release Notes

## Version 0.0.9

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