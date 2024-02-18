# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/lee-orr/dexterous_developer/compare/dexterous_developer_cli-v0.1.0...dexterous_developer_cli-v0.1.1) - 2024-02-18

### Added
- cross-platform compilation support ([#12](https://github.com/lee-orr/dexterous_developer/pull/12))

### Fixed
- fix publishing/version
- fix cargo toml duplicate description
- fix remote run asset missing
- fix bug with assets
- fix windows paths

### Other
- move build process & shared types to separate crates
- clippy + fmt + readme
- update axum
- Extract bevy crate ([#55](https://github.com/lee-orr/dexterous_developer/pull/55))
- build images for android
- Correct architecture detection on MacOS ([#39](https://github.com/lee-orr/dexterous_developer/pull/39))
- add publish=true
- bump version to 0.0.12
- Revert "update to 0.12 - start work"
- update to 0.12 - start work
- Improve Asset Reload Tests
- 27 running examples via cli ([#32](https://github.com/lee-orr/dexterous_developer/pull/32))
- Bevy main ([#31](https://github.com/lee-orr/dexterous_developer/pull/31))
- update cargo for 0.0.12-pre.0
- version bump
- Dexterous developer gui ([#18](https://github.com/lee-orr/dexterous_developer/pull/18))
- Dynamically add dylib ([#19](https://github.com/lee-orr/dexterous_developer/pull/19))
- Revert "remove need to use dylib ([#17](https://github.com/lee-orr/dexterous_developer/pull/17))"
- Revert "generate lib for compile-libs"
- generate lib for compile-libs
- remove need to use dylib ([#17](https://github.com/lee-orr/dexterous_developer/pull/17))
- update to 0.0.10
- add multiple watch directories
- f
- bump version
- start getting ready for mac... if it ever happens
- adjust
- clippy
- better target abstraction
- add compile existing
- adjust
- clippy
- add support for running existing files
- adjustments - remove zig
- add cross compilation with zig
- add keep alive, command structure
- prevent download of non-platform libs
- remove zig and xwin
- remove mold support
- adjust OS const
- update dev container
- print os striing
- sync assets
- add hashing for files
- got tests for remote
- serving
- clippy
- initial network works
- get initial serving
- Merge branch 'main' of https://github.com/lee-orr/dexterous-developer into add_remote_support
- start adding server options
- got tests for reloadables working
- update versions
- better logging
- get dynamic linker to work on linux (hopefully)
- remove cargo
- file endings
- bump versions
- update versions
- get correct things set up
- remove need for env variables when running direct
- update
- Readme & bump versions
- make it easier to call function
- update cargo versions
- add cli
- initial cli
