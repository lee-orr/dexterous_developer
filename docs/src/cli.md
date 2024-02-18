# Use with the CLI

The CLI provides a bunch of supporting functionality.

## Run

The `run` command allows you to launch a package in hot reload mode.

If you are working in a non-workspace package, you can just run `dexterous_developer_cli run`.
If you are working in a workspace with multiple libraries set up, you will need to specify the package containing your game with `dexterous_developer_cli run -p PACKAGE_NAME`.
If you want to enable or disable features, use `--features` to add the ones you want. Note that "bevy/dynamic_linking" and "hot_internal" will always be added, since they are required for the reloading capacity to work.
Another option is to use `--example EXAMPLE_NAME` - which will run the example as hot-reloadable, assuming the example is set up as a dylib. Note - this does not work if the crate itself is set as a dylib - so it's best to rely on the CLI's ability to use a temporary Cargo.toml when needed.

## Serve

The `serve` commands sets up a hot-reload build server, allowing you to connect to it via the `remote` command on another machine or serve from a dev container and run the application on the host. Currently it only supports cross compiling from Linux to Windows, otherwise both devices must be of the same platform.

## Remote

This is the compliment to the `serve` command.

## Install Cross

The `install-cross` installs the rust targets required for cross compilation. If you want to use a MacOS target, you need to provide the URL of a macos sdk. All cross compilation is based on [Cross](https://github.com/cross-rs/cross) - and so requires either docker or podman.

## Run Existing & Compile Libs

The `compile-libs` command creates the same libraries as the compiler for "serve", while `run-existing` can take a directory with the appropriate libraries and run it. This is mainly there to allow testing cross-platform builds in CI, but can also be used to run the most-recently served version of the application without re-connecting to the server.
