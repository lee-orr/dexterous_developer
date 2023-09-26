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
    reloadable_element_policy: ReloadableElementPolicy::OneOfAll(KeyCode::F3),
    reloadable_element_selection: None,
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

## Reloadable Element Policy

This allows you to only enable more complex reload (cleanup/setup functions and/or serialization/deserialization) for one "Reloadable Element" at a time. A reloadable element is the function that sets up all the reloadable portions of the app - in the example, it's shown as:

```rust
#[dexterous_developer_setup]
fn reloadable(app: &mut ReloadableAppContents) {
    ...
}
```

In your app, you can treat these similarly to plugins, and have more than one of them. However - their names mustn't conflict on a global scale, and to help with that you can pass in an additional parameter to the macro. Here is a little mock up:

```rust

#[hot_bevy_main]
pub fn bevy_main(initial_plugins: impl InitialPlugins) {
    App::new()
    ...
        .setup_reloadable_elements::<first::reloadable>()
        .setup_reloadable_elements::<second::reloadable>()
    ...
}

mod first {
        #[dexterous_developer_setup(first_reloadable)]
    fn reloadable(app: &mut ReloadableAppContents) {
        ...
    }

}

mod second {

    #[dexterous_developer_setup(second_reloadable)]
    fn reloadable(app: &mut ReloadableAppContents) {
        ...
    }
}


```

The Reloadable Element Policy allows you to determine how, and if, you want to handle reloading each of them. Specifically - it lets you decide to only fully re-load one of them, while others will only re-load updated systems but not run any setup/cleanup or serialization/deserialization. This is useful if you are working on a specific element, for example the UI, that requires running a setup function to re-build it - but where you don't want to necessarily re-run the setup for other systems.

There are 3 possible values:

- All - with this policy, all elements are always reloadable - and no toggling is available.
- OneOfAll(KeyCode) - with this policy you provide a key that you can use to cycle between all the reloadable elements in your project.
- OneOfList(keyCode, &'static [&'static str]) - with this policy you can provide a hard-coded subset of elements you want to allow cycling between. This is mostly useful for situations where your application is complex enough that there are too many reloadable elements to cycle through, but you might want to alternate work between a subset of them.

## Reloadable Element Selection

This is an optional value that defaults to "None". As a rule, it is recommended to leave it as is. However, if necessary - you can use to to pre-set a specific reloadable element that will be focused on. If the policy allows for cycling between elements, that will still be possible - it just changes the initial default.
