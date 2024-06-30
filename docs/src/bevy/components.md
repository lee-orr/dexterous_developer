# Components

Components can be handled in a few ways as well.

## Serializable Components

If you have a component that you want to serialize and de-serialize, allowing you to maintain it's state while evolving it's schema.

You set up the component as serializable by calling `app.register_serializable_component<C: Component + ReplacableType>()` within a reloadable scope.

- using `serde` and implementing `SerializableType`. This approach relies on `rmp_serde` to serialize and deserialize the resource.

  ```rust
    #[derive(Component, Serialize, Deserialize)]
    struct MyComponent(String);

    impl SerializableType for MyComponent {
        fn get_type_name() -> &'static str {
            "MyComponent
        }
    }
  ```

- implementing `ReplacableType` yourself:

  ```rust
  #[derive(Component)]
  struct MyComponent(String);

  impl ReplacableType for MyComponent {
    fn get_type_name() -> &'static str {
        "MyComponent"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        Ok(self.0.as_bytes().to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        Ok(Self(std::str::from_utf8(val))?))
    }
  }
  ```

## Clear on Reload

Alternatively, you may want to fully remove any entities that have a given component upon reload. To do so, you just need to call `app.clear_marked_on_reload::<C: Component>()` from within a reloadable scope. Whenever a reload occurs, all entities with the component will be removed. Note - this will also despawn any descendents.

## Reset Setup

Finally, you may wish to both clear all entities with a component and run a setup function after reload - for example, to re-build a UI. You can do so in one of 2 ways - calling `app.reset_setup<C: Component, M>(systems)`, which will clear all entities with the component (and their descendents) and then run the systems on every reload, or calling `app.reset_setup_in_state<C: Component, S: States, M>(state: S, systems)` which will despawn on every reload, but only run the setup if you are in a given state. This will despawn marked systems on exit, similar to [`enable_state_scoped_entities`](https://docs.rs/bevy/0.14.0-rc.4/bevy/state/app/trait.AppExtStates.html#tymethod.enable_state_scoped_entities).
