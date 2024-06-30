# How Dexterous Developer Works

Dexterous Developer is designed to be a modular hot reload system, and it does so by separating into a set of components and standerdizing the communication between them.

If you don't need additional customization, you only need to care about the adapter for your framework or the `dexterous_developer_instance` crate if no adapter exists, along with using the provided CLIs.

## Instance Side Components

These components run within the reloaded codebase, and provide the the main point of integration with your own code.

- **Instances** - these provide the lower-level elements that the adapters build on, such as providing hooks for knowing an update is available, replacing the current running instance, and ways to call elements through the reload boundary.
- **Adapters** - these are build specifically to embed within another framework or known pattern for managing a persistent application. Hot Reload only makes sense in long-running processes, ideally ones that repeat regularly such as UI, Graphics, Simulations, and Web Servers. As a rule, they often have some form of loop or trigger-based execution pattern, a way to persist state across executions, and some known methods of interacting with the outside world. The Adapter provides a way to integrate directly with a specitic framework, while providing some higher-level features on top. At this time, `bevy_dexterous_developer` is the only first-party adapter we provide.

## Client Side Components

These components are used to execute the instance, download updates to the libraries or assets, and basically any element that needs to be managed externally to your application and not re-loaded along with it.

- **Runners** - these provide the necessary logic to handle executing the application using a particular reload approach. At this time, dynamic libraries are the only supported approach - provided by `dexterous_deverloper_dylib_runner`.
- **The Runner CLI** - this provides a pre-made way to execute reloadable applications via the command line, so you don't need to set up a runner in your own codebase.

## Server Side Components

These components are used to watch the codebase for changes, manage builds, and provide the runners with access to the associated code and assets.

- **Builders** - these handle triggering the cargo builds themselves, processing the output, and collecting library dependencies.
- **Watchers** - these handle watching the file system for changes, allowing for notifications in changes to assets (non-compiled files) and sending out the triggers for re-builds when the code itself changes.
- **Manager** - the manager handles setting up the chosen watcher & builders for each available target, and sets up a server for the runner to connect to.
- **The Server CLI** - this provides a pre-made way to run a build server using the default builders & watcher.

## Shared Types

The `dexterous_developer_types` crate provides types that are shared across components, both on the client and the server.
