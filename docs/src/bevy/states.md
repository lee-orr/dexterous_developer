# States

## Setting Up States

The states you define within a reloadable scope need to be either `SerializableType` or `ReplacableType` - but otherwise their behaviour should match native bevy states. Note that no transition gets triggered upon the initial change - so if you changed the logic for a sub states existance or a computed state's compute function you may need to trigger the transitions manually..

For `Freely Mutable States`, call `app.init_state<S: FreelyMutableState + ReplacableType + Default>()` or `app.insert_state<S: FreelyMutableState + ReplacableType>(initial: S)` within a reloadable scope.

For `Sub States`, call `app.add_sub_state<S: SubStates + ReplacableTypes>()`, and for `Computed States` call `app.add_computed_state<S: ComputedState + ReplacableTypes>()`.

> ![Note]
> ReplacableTypes is required for computed states to avoid having to re-trigger transitions while retaining the current value. Otherwise, we would need to `Exit` all reloadable states before a reload and re-enter the new states after - which could re-set elements of state you want to keep.
> Instead, handle any such re-sets with `app.reset_setup_in_state` with the help of marker components.

You can either implement `SerializableType`:

```rust

#[derive(States, Debug, Default, Hash, PartialEq, Eq, Clone, Serialize, Deserialize)]
enum MyState {
    #[default]
    InitialState,
    AnotherState
}

impl SerializableType for MyState {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }
}
```

or `ReplacableType` directly:

```rust

#[derive(States, Debug, Default, Hash, PartialEq, Eq, Clone)]
enum MyState {
    #[default]
    InitialState,
    AnotherState
}

impl ReplacableType for MyState {
    fn get_type_name() -> &'static str {
        "MySerializableResource"
    }

    fn to_vec(&self) -> bevy_dexterous_developer::Result<Vec<u8>> {
        let value = match self {
            MyState::InitialState => [0],
            MyState::AnotherState => [1],
        };
        Ok(value.to_vec())
    }

    fn from_slice(val: &[u8]) -> bevy_dexterous_developer::Result<Self> {
        let value = if let Some(val) = val.get(0) {
            if *val == 1 {
                MyState::AnotherState
            } else {
                MyState::InitialState
            }
        } else {
            MyState::InitialState
        };
        Ok(value)
    }
}
```

## State Scoped Entities

For most contexts, you will want to use `reset_setup_in_state<C: Component, S: States, M>(state: S, systems)` instead - this combines running systems in `OnEnter` or upon reload, and despawning entities marked with `C` (and their descendents) `OnExit` or `OnReload`.

However, if you want a `Scoped` entity that doesn't despwan, but instead remains so long as you are in the given state across reloads, you can also use `enable_state_scoped_entities<S: States + ReplacableType>()` just as you would outside the reloadable scope, and then add `StateScoped(value: S)` to any entities you care about here.
