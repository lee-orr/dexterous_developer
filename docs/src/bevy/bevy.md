# Bevy

[Bevy](https://bevyengine.org/) is an open-source game engine built in Rust, and is the origianl usecase for Dexterous Developer. As a result, it's adapter provides a wide set of supported features.

## Supported Features

- [System Replacement and Registration](./system_replacement.md)
- [Reloading and Resetting Resources](./resources.md)
- [Reloading Components and Re-Running Setup Functions](./components.md)
- [Replacing Events](./events.md)
- [Reloading States](./states.md)

## Warning

Note that you cannot use Resources, Components, Events or States registered in a reloadable scope outside of the elements systems tied to that scope. Otherwise, you run the risk of running into undefined behaviour.

For example:

```rust
reloadable_main!( bevy_main(initial_plugins) {
    App::new()
        .add_plugins(initial_plugins.initialize::<DefaultPlugins>())
        .setup_reloadable_elements::<reloadable>()
         // This will fail after reload since it queries MyComponent, which
         // is registered as a serializable component.
        .add_systems(Update, count_components)
        
        .run();
});

#[derive(Component, Serialize, Deserialize)]
struct MyComponent;

impl SerializableType for MyComponent {
    fn get_type_name() -> &'static str {
        "MyComponent"
    }
}

reloadable_scope!(reloadable(app) {
    app
        .register_serializable_component::<Myomponent>()
        // Instead, place any systems that rely on elements that are reloadable
        // within the reloadable scope.
        .add_systems(Update, count_components);
})

fn count_components(query: Query<&MyComponent>) {
    /../
}

```

This is because reloadable elements will be replaced with potentailly different types, but the systems registered in `reloadable_main` will keep using the old version.
