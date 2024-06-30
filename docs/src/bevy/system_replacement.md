# System Replacement and Registration

The simplest feature of `dexterous_developer` is the ability to register, remove and replace systems. To do so, all you need to do is use `.add_systems` within a `reloadable_scope!`:

```rust
    reloadable_scope!(reloadable(app) {
        app
            .add_systems(Update, my_dope_system);
    });
```

Any system added from within a reloadable scope will be removed before the reloadable scope function runs to set up the replacement systems.
