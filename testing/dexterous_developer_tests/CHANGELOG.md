# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/lee-orr/dexterous_developer/releases/tag/dexterous_developer_tests-v0.1.0) - 2024-02-18

### Added
- cross-platform compilation support ([#12](https://github.com/lee-orr/dexterous_developer/pull/12))
- added mold support on linux via prefer_mold option

### Fixed
- fix test
- fix has_updated in tests

### Other
- update bevy 0.13
- move build process & shared types to separate crates
- Extract bevy crate ([#55](https://github.com/lee-orr/dexterous_developer/pull/55))
- Correct architecture detection on MacOS ([#39](https://github.com/lee-orr/dexterous_developer/pull/39))
- Improve Asset Reload Tests
- 27 running examples via cli ([#32](https://github.com/lee-orr/dexterous_developer/pull/32))
- 22 events ([#26](https://github.com/lee-orr/dexterous_developer/pull/26))
- State-support ([#25](https://github.com/lee-orr/dexterous_developer/pull/25))
- ci fix
- add tests for clearning components, running setup functions, and clearing components + runing a setup function in a specific state
- split reloadable tests into individual runs
- documentation
- add cross-compile test
- update
- a
- build tester and cli before running tests
- temporarily remove non-remote tests for ci debugging
- add keep alive, command structure
- remove mold support
- remove hidden examples
- update dev container
- sync assets
- add hashing for files
- got tests for remote
- adjusted exit signal
- adjust logging
- got tests for reloadables working
- ensure hashed files aren't replaced
- update tests
- clippy + fmt
- got compilation on windows working reliably again
- println
- better logging
- ensure watching before ready
- get dynamic linker to work on linux (hopefully)
- update ci
- ensure exits happen
- remove cli snadbox unused dependency
- ensure CLI is only build once
- adjust
- prevent infinite loop waiting for non-existing files
- clippy, fmt, check for clang and lld, ensure pointers get followed.
- remove cargo
- file endings
- got hot reload tests working
- added tests for updating files in hot reload
- cleanup
- got passing run hot
- better printout from utils
- clippy
- added cold test
