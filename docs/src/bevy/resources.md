# Resources

Resources can be reloaded in a variety of ways.

## Resetting Resources

If you have a resource that you want to re-set when a reload occurs, you can do so using either `app.reset_resource::<R: Resource + Default>()` or `app.reset_resource_to_value::<R: Resource>(value: R)` within a reloadable scope. This will cause the resource to be removed and re-initialized when new coad is loaded.

## Serializable Resources

If you have a resource that you want to serialize and de-serialize, allowing you to maintain it's state while evolving it's schema.

You initialize the resource by using either `app.init_serializable_resource::<R: ReplacableType + Resource + Default>()` or `app.insert_serializable_resource::<R: ReplacableType + Resource>(initializer: impl 'static + Send + Sync + Fn() -> R)`

You can also mark a resource type as serializable without actively adding a copy of it to the application using `app.register_serializable_resource::<R: ReplacableType + Resource>()`. This will only serialize/deserialize the resource if it existins at the time of the reload.

- using `serde` and implementing `SerializableResource`. This approach relies on `rmp_serde` to serialize and deserialize the resource.

  ```rust
    #[derive(Resource, Serialize, Deserialize)]
    struct MyResource(String);

    impl SerializableType for MyResource {
        fn get_type_name() -> &'static str {
            "MyResource
        }
    }
  ```

- implementing `ReplacableType` yourself:

  ```rust
  #[derive(Resource)]
  struct MyResource(String);

  impl ReplacableType for MyResource {
    fn get_type_name() -> &'static str {
        "MyResource"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        Ok(self.0.as_bytes().to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        Ok(Self(std::str::from_utf8(val))?))
    }
  }
  ```
