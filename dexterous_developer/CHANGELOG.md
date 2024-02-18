# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/lee-orr/dexterous_developer/compare/dexterous_developer-v0.1.0...dexterous_developer-v0.1.1) - 2024-02-18

### Fixed
- fix publishing/version
- fixed allocation issue
- fix transfer of path between proesses
- fix unneeded borrow
- fix links

### Other
- Extract bevy crate ([#55](https://github.com/lee-orr/dexterous_developer/pull/55))
- Correct architecture detection on MacOS ([#39](https://github.com/lee-orr/dexterous_developer/pull/39))
- add publish=true
- bump version to 0.0.12
- Revert "update to 0.12 - start work"
- update to 0.12 - start work
- update cargo for 0.0.12-pre.0
- version bump
- update to 0.0.10
- bump version
- initial network works
- clippy + fmt
- only use dynamic lib when hot_internal is enabled
- got tests for reloadables working
- update versions
- ensure hashed files aren't replaced
- update tests
- got compilation on windows working reliably again
- cstring again
- clippy + string lossy
- use &CStr for transfer of info
- better logging
- clippy + fmt
- remove rpath
- windows fixes
- format
- get dynamic linker to work on linux (hopefully)
- prevent infinite loop waiting for non-existing files
- clippy, fmt, check for clang and lld, ensure pointers get followed.
- clippy + fmt
- remove cargo
- file endings
- got hot reload tests working
- bump versions
- remove loader macro
- update versions
- move bevy support into module, clean up cfg flags
- additional improvements to env
- encourage nightly
- call from library holder
- better set up
- use cargo directly
- clippy , fmt, cleanup
- simplify dependencies
- cluppy + fmt
- remove need for env variables when running direct
- add impl for App
- user merged environments
- update
- Readme & bump versions
- update cli
- remove the dexterous developer winit fork
- make it easier to call function
- update cargo versions
- add cli
- initial cli
- clippy + fmt
- update to InitialPlugins trait
- simplify
- Add safety comments and improve the error experience of setup_reloadable_app
- more renames and cleanup
