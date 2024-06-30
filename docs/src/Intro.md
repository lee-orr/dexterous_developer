# Introduction

Dexterous Developer is a modular hot-reload system for Rust. At this point, it provides an adapter for Bevy - but more adapters will be added in the near future.

## Features

- A CLI for building & running reloadable rust projects, including over the network (cross-device)
- The ability to serialize/deserialize elements, allowing the evolution of schemas over time
- Only includes any hot reload capacity in your build when you explicitly enable it - such as by using the CLI launcher
- The capacity to create adapters for additional frameworks, allowing you to use Dexterous Developer tooling with other tools.
- Includes a first-party Bevy adapter
- Works on Windows, Linux, and MacOS 
- On Linux, can be used to develop within a dev container while running on the main OS, enabling use of dev containers for games & other GUI apps.

### Bevy Specific

- Define the reloadable areas of your game explicitly - which can include systems, components, states, events and resources (w/ some limitations)
- Reset resources to a default or pre-determined value upon reload
- Serialize/deserialize your reloadable resources & components, allowing you to evolve their schemas so long as they are compatible with the de-serializer (using rmp_serde)
- Mark entities to get removed on hot reload
- Run systems after hot-reload
- Create functions to set up & tear down upon either entering/exiting a state or on hot reload

## Additional Resources

We also have [API Docs](https://lee-orr.github.io/dexterous_developer/doc/dexterous_developer/index.html)

## Inspiration

This project was inspired by [DGriffin91's Ridiculous bevy hot reloading](https://github.com/DGriffin91/ridiculous_bevy_hot_reloading).
