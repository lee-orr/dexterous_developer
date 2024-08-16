# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## `dexterous_developer` - [0.4.0-alpha.0](https://github.com/lee-orr/dexterous_developer/compare/dexterous_developer-v0.3.0..v0.4.0-aplha.0)

- Replace "Simple Builder" with new "Default Builder" that supports building binary crates
- Fix "Failed to find dll" errors causing dexterous_developer to fail on windows (#64)
- Keep all versions of reloaded crate in memory to avoid issues with pointing to elements in an unloaded library
- Fix bug where occasionally the watcher wouldn't trigger a rebuild
- Fix bug where build errors could prevent future build attempts
- add more options to Dexterous.toml

## `dexterous_developer` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer-v0.2.0...dexterous_developer-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `bevy_dexterous_developer` - [0.3.0](https://github.com/me/my-proj/compare/bevy_dexterous_developer-v0.2.0...bevy_dexterous_developer-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `bevy_dexterous_developer_library` - [0.3.0](https://github.com/me/my-proj/compare/bevy_dexterous_developer_library-v0.2.0...bevy_dexterous_developer_library-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `bevy_dexterous_developer_dynamic` - [0.3.0](https://github.com/me/my-proj/compare/bevy_dexterous_developer_dynamic-v0.2.0...bevy_dexterous_developer_dynamic-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `dexterous_developer_cli` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_cli-v0.2.0...dexterous_developer_cli-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `dexterous_developer_dynamic` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_dynamic-v0.2.0...dexterous_developer_dynamic-v0.3.0) - 2024-07-04

### Other
- *(bevy)* update to bevy 0.14

## `dexterous_developer_instance` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_instance-v0.2.0...dexterous_developer_instance-v0.3.0) - 2024-07-04

### Added
- cross-platform compilation support ([#12](https://github.com/lee-orr/dexterous_developer/pull/12))

### Fixed
- fix markdown lint
- fix publishing/version

### Other
- *(bevy)* update to bevy 0.14
- *(workspace)* fix doc version
- bump version and changelog
- fix bevy version in docs
- *(workspace)* Fix version used in docs
- *(workspace)* adjusting docs further
- *(workspace)* :memo: Adjust docs release process to generate multiple versions
- *(workspace)* add pre-release book
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings
- changelog
- update
- remove state for now
- instructions on running with hot reload
- add version alert
- update readme
- update docs
- rename "hot_internal" to "hot"
- rename "hot" feature to "runner"
- rename `dexterous_developer_internal` to `dexterous_developer_instance`
- add bevy quick start
- better readme
- remove macro crate, add `macro_rules!` macros to bevy_dexterous_developer
- Update README.md
- bump version
- Update README.md
- update readme
- update bevy 0.13
- clippy + fmt + readme
- Extract bevy crate ([#55](https://github.com/lee-orr/dexterous_developer/pull/55))
- Correct architecture detection on MacOS ([#39](https://github.com/lee-orr/dexterous_developer/pull/39))
- bump version to 0.0.12
- Revert "update to 0.12 - start work"
- Revert "Merge branch 'main' of https://github.com/lee-orr/dexterous-developer"
- Merge branch 'main' of https://github.com/lee-orr/dexterous-developer
- Update README.md
- 27 running examples via cli ([#32](https://github.com/lee-orr/dexterous_developer/pull/32))
- 22 events ([#26](https://github.com/lee-orr/dexterous_developer/pull/26))
- Merge branches 'main' and 'main' of https://github.com/lee-orr/dexterous-developer
- update cargo for 0.0.12-pre.0
- correct markdown, release notes
- Dynamically add dylib ([#19](https://github.com/lee-orr/dexterous_developer/pull/19))
- Revert "adjust docs"
- adjust docs
- update to 0.0.10
- documentation
- bump version
- update version number
- file endings
- bump versions
- update versions
- readme + clippy
- update
- Readme & bump versions
- remove the dexterous developer winit fork
- update readme
- update docs for 0,.0.3
- update to InitialPlugins trait
- improve docs
- update readme
- more renames and cleanup
- add readme info

## `dexterous_developer_manager` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_manager-v0.2.0...dexterous_developer_manager-v0.3.0) - 2024-07-04

### Fixed
- fix assets

### Other
- bump version and changelog
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings
- changelog
- update
- update dashma[
- clippy, fmt, test
- basic cli tests
- build test works fast
- start work towards setting up automated tests
- format, clippy, cleanup printlnines
- update version
- moved bevy specific crates into a subdirectory of bevy_dexterous_developer
- get simple cli test to run
- clippy + fmt
- recursive dependencies
- adjustments to server/builder
- build simple watcher
- fmt + clippy
- add watcher
- start implementing simple builder
- move builder & current state to have an optional root lib
- move to utf8path
- add file target
- add connect to target
- improve manager & builder APIs
- add server feature to manager
- move builder types to builder repo
- start work on manager

## `dexterous_developer_builder` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_builder-v0.2.0...dexterous_developer_builder-v0.3.0) - 2024-07-04

### Fixed
- *(workspace)* swap from INotifyWatcher to RecommendedWatcher
- *(workspace)* import INotifyWatcher by it's full path
- fix assets

### Other
- bump version and changelog
- update
- update dashma[
- clippy, fmt, test
- build test works fast
- increase timeout for build test
- adjust test time for builder
- format, clippy, cleanup printlnines
- test builder - actual build
- add watcher tests
- format
- remove unused file
- add current build state tests
- update version
- clippy + fmt
- moved bevy specific crates into a subdirectory of bevy_dexterous_developer
- continue work
- clippy + fmt
- get simple cli test to run
- setup callbacks and all
- start setting up for running dynamic libraries
- clippy + fmt
- recursive dependencies
- adjustments to server/builder
- build simple watcher
- fmt + clippy
- add watcher
- start implementing simple builder
- move builder & current state to have an optional root lib
- move to utf8path
- clippy + fmt
- add dylib runner crate, comment out stuff in cli, builder & internal
- add file target
- add connect to target
- improve manager & builder APIs
- move builder types to builder repo
- fmt + clippy
- remove macro crate, add `macro_rules!` macros to bevy_dexterous_developer
- update release
- avoid direct implementation of ToString
- move build process & shared types to separate crates

## `dexterous_developer_types` - [0.3.0](https://github.com/me/my-proj/compare/dexterous_developer_types-v0.2.0...dexterous_developer_types-v0.3.0) - 2024-07-04

### Other
- update
- test builder - actual build
- move config to types
- update config extraction
- start setting up new HotReloadInfo struct
- start setting up for running dynamic libraries
- clippy + fmt
- recursive dependencies
- adjustments to server/builder
- build simple watcher
- add watcher
- move builder & current state to have an optional root lib
- clippy + fmt + feature_list
- generate build settings from config
- move to utf8path
- clippy + fmt and adjust cli
- clippy + fmt
- runner runs with env variables
- add file target
- add connect to target
- improve manager & builder APIs
- remove tokio from types
- move builder types to builder repo
- start work on manager
- move build process & shared types to separate crates

## `bevy_dexterous_developer_library` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/bevy_dexterous_developer_library-v0.2.0...bevy_dexterous_developer_library-v0.3.0-pre.2) - 2024-07-02

### Added
- *(bevy)* :sparkles: state scoped
- *(bevy)* :sparkles: add computed state
- *(bevy)* :sparkles: Initial Sub-State Support

### Fixed
- *(bevy)* simplify resource insertion

### Other
- *(workspace)* clippy + fmt
- *(workspace)* format
- initial state support
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings

## `bevy_dexterous_developer_dynamic` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/bevy_dexterous_developer_dynamic-v0.2.0...bevy_dexterous_developer_dynamic-v0.3.0-pre.2) - 2024-07-02

### Other
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings

## `dexterous_developer_test_utils` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/dexterous_developer_test_utils-v0.2.0...dexterous_developer_test_utils-v0.3.0-pre.2) - 2024-07-02

### Added
- *(bevy)* :sparkles: state scoped

### Fixed
- *(bevy)* simplify resource insertion

### Other
- *(workspace)* clippy + fmt
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings
- changelog
- update
- clippy, fmt, test
- test improvements
- basic cli tests
- build all examples & binaries before test
- build test works fast

## `dexterous_developer_dylib_runner` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/dexterous_developer_dylib_runner-v0.2.0...dexterous_developer_dylib_runner-v0.3.0-pre.2) - 2024-07-02

### Fixed
- fix assets

### Other
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings
- changelog
- update
- rename "hot" feature to "runner"
- rename `dexterous_developer_internal` to `dexterous_developer_instance`
- update dashma[
- basic cli tests
- build test works fast
- remove dylib test runner
- adjust and add simple_cli_loaded_test example
- remove untethered
- small fixes
- split runner and connection
- reorganize files for dylib runner
- format, clippy, cleanup printlnines
- update version
- clippy + fmt
- clippy + fmt
- Reloading! ([#59](https://github.com/lee-orr/dexterous_developer/pull/59))
- moved bevy specific crates into a subdirectory of bevy_dexterous_developer
- continue work
- get library to call update again
- improvements to loading
- clippy + fmt
- get simple cli test to run
- setup callbacks and all
- start setting up for running dynamic libraries
- recursive dependencies
- move to utf8path
- clippy + fmt
- start setting up runner
- clippy + fmt
- add dylib runner crate, comment out stuff in cli, builder & internal

## `dexterous_developer_instance` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/dexterous_developer_instance-v0.2.0...dexterous_developer_instance-v0.3.0-pre.2) - 2024-07-02

### Other
- fix bevy version in docs
- *(workspace)* Fix version used in docs
- *(workspace)* adjusting docs further
- *(workspace)* :memo: Adjust docs release process to generate multiple versions
- *(workspace)* add pre-release book
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings

## `dexterous_developer_manager` - [0.3.0-pre.2](https://github.com/me/my-proj/compare/dexterous_developer_manager-v0.2.0...dexterous_developer_manager-v0.3.0-pre.2) - 2024-07-02

### Fixed
- fix assets

### Other
- *(workspace)* :building_construction: adjust ordering of crates in workspace and release-plz settings
- changelog
- update
- update dashma[
- clippy, fmt, test
- basic cli tests
- build test works fast
- start work towards setting up automated tests
- format, clippy, cleanup printlnines
- update version
- moved bevy specific crates into a subdirectory of bevy_dexterous_developer
- get simple cli test to run
- clippy + fmt
- recursive dependencies
- adjustments to server/builder
- build simple watcher
- fmt + clippy
- add watcher
- start implementing simple builder
- move builder & current state to have an optional root lib
- move to utf8path
- add file target
- add connect to target
- improve manager & builder APIs
- add server feature to manager
- move builder types to builder repo
- start work on manager
# Release Notes

## Version 0.2.0

- extract bevy support to bevy_dexterous_developer crate
- extract compilation & watcher to dexterous_developer_builder crate
- temporarily disable remote CLI tests

## Version 0.1.0

- fix cli compilation error on Apple Silicone - <https://github.com/lee-orr/dexterous_developer/issues/38>
- fix ci install of LLVM for mac
- temporary fix for segfault on Apple Silicone devices - <https://github.com/bevyengine/bevy/issues/10524>

## Version 0.0.12

- add support for `Bevy 0.12`
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
