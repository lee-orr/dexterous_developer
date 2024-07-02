# States

Different types of states have different types of interactions with Dexterous Developer. But it's important to note that by default, they will not trigger any of their associated transitions upon reload.

## Standard States

For "standard" states, the main interaction lies in them being serialized and de-serialized, like any other resource.

To use them, you need to implement `SerializableType` or `ReplacableType` for the state, and call either `app.init_state<S: FreelyMutableState + ReplacableType + Default>()` or `app.insert_state<S: FreelyMutableState + ReplacableType>(initial: S)`.
