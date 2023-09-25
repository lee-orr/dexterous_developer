# Reload Settings

This is where we'll be adding settings that can be adjusted during runtime when you are hot reloading.
To add reload settings, insert the `ReloadSettings` resource in your application:

```rust
.insert_resource(ReloadSettings::default())
```

which is equivalent to:

```rust
.insert_resource(ReloadSettings {
    display_update_time: true,
    manual_reload: Some(KeyCode::F2),
    toggle_reload_mode: Some(KeyCode::F1),
    reload_mode: ReloadMode::Full,
})
```

## Display Update Time

This setting will display the most recent update time in the window title for your game, letting you know whether the reload has happened yet. This is useful for subtle changes or situations where you are unsure whether things worked, where you'd be able to look at that timestamp and determine if it's recent enough to be your most recent change.

## Reload Mode

The reload mode controls the specific elements that get re-loaded:

- Full (the default) - this runs a full hot reload, including systems, reloadable resources and components, and cleanup/setup functions
- SystemAndSetup - this reloads systems and runs cleanup/setup functions, but doesn't re-set, serialize or de-serialize resources and components
- SystemOnly - this reloads systems and does nothing else

## Manual Reload

This allows you to set a key (defaults to F2) that will trigger a reload based on the current reload mode - without needing to make a code change. This is useful if you want to manually re-set resources or trigger setup functions.

## Toggle Reload Mode

This allows you to set a key (defaults to F1) that will cycle between reload modes.
