# Reload Settings

This is where we'll be adding settings that can be adjusted during runtime when you are hot reloading. At the moment, there is only one setting available, but more might be added in the future.

To add reload settings, insert the `ReloadSettings` resource in your application:

```rust
.insert_resource(ReloadSettings {
            display_update_time: true,
        })
```

## Display Update Time

This setting will display the most recent update time in the window title for your game, letting you know whether the reload has happened yet. This is useful for subtle changes or situations where you are unsure whether things worked, where you'd be able to look at that timestamp and determine if it's recent enough to be your most recent change.
