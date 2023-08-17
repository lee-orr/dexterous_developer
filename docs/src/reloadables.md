# Adding Reloadables to your Plugins

You can add reloadable portions to your application within any plugin, but they have to be explicitly segmented and defined. Within a plugin, you would add a reloadable section like so:

```rust

impl Plugin for MyPlugin {
    fn build(&self, app: &mut App) {
        app
            .setup_reloadable_elements::<reloadable>();
    }
}

#[dexterous_developer_setup]
fn reloadable(app: &mut ReloadableAppContents) {
    app
        .add_systems(Update, this_system_will_reload);
}

```

Here, we are defining a setup function `reloadable`, and adding it to our app in the plugin builder. This function will be called any time the library is reloaded.

In the background, the plugin runs `cargo watch` on the source directory of the project, and whenever it re-builds we will load a new version of the library.

## Types of Reloadables

### Systems

The simplest, and most versitile, type of reloadable is a simple system. This can be any system - we just register it in a special way allowing us to remove them when thge library is replaced.

These are added using `.add_systems`, exactly like you would add a system to a normal bevy app!

#### Setup Systems

These are special systems, that are supposed to set things up - which might need to be re-done upon a reload. The classic example is a game UI, which you might want to re-build after a reload. There are a few helpers for these types of systems

First, we can clear all entites that have a marker component and then run our setup function using `.reset_setup<Component>(systems_go_here)`.
And if we want it to only happen on entering a specific state (or re-loading while within that state), we can use `.reset_setup_in_state<Component>(state, systems)`.
Alternatively, if we just want to clear stuff out on a reload, we can use a marker component and call `.clear_marked_on_reload<Component>()`.

## Resources

Reloading resources is a little more complex - so we have a few variations

### Reset on Reload

If you want to reset a resource when the library reloads, you can use either `.reset_resource<Resource>()` which uses it's default value, or `.reset_resource_to_value(value)` which uses a value you provide.

### Replaceable Resources

If you want to be able to iterate on the structure of a resource, but maintain it's existing data via serialization, you can use a `ReplacableResource`. To do so you need to implement the `ReplacableResource` trait on your type:

```rust
#[derive(Resource, Serialize, Deserialize, Default)]
struct MyResource

impl ReplacableResource {
    fn get_type_name() -> &'static str {
        "my_resource"
    }
}
```

Then, you can register it using `.insert_replacable_resource<ReplacableResource>()`. This will cause the resource to be serialized before the library is reloaded, and replaced with a new version after reload. Since serialization is done using msgpack, it should be able to cope with adding new fields or removing old ones - but keep in mind the way serde handles that kind of stuff.

## Replacable Components

The last type of reloadable are replacable components. These function like replacable resources, but involve replacing components on various entities. Here you implement the `ReplacableComponent` trait:

```rust
#[derive(Component, Serialize, Deserialize, Default)]
struct MyComponent

impl ReplacableComponent {
    fn get_type_name() -> &'static str {
        "my_component"
    }
}

```

And then register it using `.register_replacable_component<ReplacableComponent>()`.
