# Hot Reload

Heavily inspired by <https://github.com/DGriffin91/ridiculous_bevy_hot_reloading>, currently very much a work in progress/experimental.

## Features

- Define hot reloaded areas of your game, which can arbitrary systems, resources and components (w/ some limitations)
- reset resources to default or a pre-determined value upon reload
- serialize/deserialize specific resources & components (using rmp_serde) - allowing for evolving their schema so long as they are compatible with the de-serializer
- mark entities to get removed on hot reload
- run systems after a hot reload
- create a setup that will run upon entering a state or reloading within that state, and will clear marked entities upon leaving the state or on reload (great for UI work).
- bypass hot reload by default - it only includes minimal shims unless you pass in the "hot" feature to enable hot reloading

## Missing/WIP/Issues

- known to work on Windows, but likely not on other platforms - as it relies on a fork of bevy_winit with a specific flag set...I want to either get rid of this forking or at least make sure other platforms can work
  - it is unlikely it'll ever work on WASM...
- finding a way to allow execution of examples directly with the hot reloader - basically moving it to act like a runner (akin to wasm-server-runner)
- events and states still need to be pre-defined
- will likely not work that well with anything that requires main thread access
