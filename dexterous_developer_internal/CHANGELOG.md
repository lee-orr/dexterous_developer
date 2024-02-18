# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/lee-orr/dexterous_developer/compare/dexterous_developer_internal-v0.1.0...dexterous_developer_internal-v0.1.1) - 2024-02-18

### Added
- cross-platform compilation support ([#12](https://github.com/lee-orr/dexterous_developer/pull/12))
- add support for mold using an env variable ([#13](https://github.com/lee-orr/dexterous_developer/pull/13))
- added mold support on linux via prefer_mold option

### Fixed
- fix publishing/version
- fix bug with assets
- fix windows
- fix

### Other
- move build process & shared types to separate crates
- clippy
- remove need for nightly
- update which crate
- Extract bevy crate ([#55](https://github.com/lee-orr/dexterous_developer/pull/55))
- build images for android
- Correct architecture detection on MacOS ([#39](https://github.com/lee-orr/dexterous_developer/pull/39))
- add publish=true
- bump version to 0.0.12
- working 0.12
- setup for idea ide
- Revert "update to 0.12 - start work"
- update to 0.12 - start work
- Improve Asset Reload Tests
- 27 running examples via cli ([#32](https://github.com/lee-orr/dexterous_developer/pull/32))
- Bevy main ([#31](https://github.com/lee-orr/dexterous_developer/pull/31))
- 22 events ([#26](https://github.com/lee-orr/dexterous_developer/pull/26))
- Merge branches 'main' and 'main' of https://github.com/lee-orr/dexterous-developer
- update cargo for 0.0.12-pre.0
- version bump
- Reload individual elements ([#21](https://github.com/lee-orr/dexterous_developer/pull/21))
- add last update label to top of window
- Dynamically add dylib ([#19](https://github.com/lee-orr/dexterous_developer/pull/19))
- Revert "remove need to use dylib ([#17](https://github.com/lee-orr/dexterous_developer/pull/17))"
- remove need to use dylib ([#17](https://github.com/lee-orr/dexterous_developer/pull/17))
- render compilation errors
- update to 0.0.10
- add multiple watch directories
- documentation
- bump version
- start getting ready for mac... if it ever happens
- clippy
- better target abstraction
- ensure build output
- clippy
- add compile existing
- add support for running existing files
- adjustments - remove zig
- add cross compilation with zig
- add keep alive, command structure
- remove zig and xwin
- move to build providers
- remove mold support
- sync assets
- add hashing for files
- got tests for remote
- initial network works
- get initial serving
- refactor to separate out statics and actual functions
- adjust logging
- got tests for reloadables working
