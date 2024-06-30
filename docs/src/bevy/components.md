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
