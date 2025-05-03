# Hot Reload Primitives for Rust

Note that for all these types, specific framework adapters may be used/created to optimize for better performance. For example, serializing Components in the bevy ecosystem may be better performed in a manner that can retain cache locality rather than the default approaches.

## Structs & Enums

### Persistent Types

these are types that do not change during development, such as types from external crates, or that you do not expect would require regular iteration on their structure. Note that adjusting trait implementations are separate (see Implicitly Reloaded Functions below).

### Resetting Types

these are types that revert to a default value upon reload - for example, it may not be necessary to retain focus state in a UI upon reload, and so that value would reset to it's default.

### Serializable Types

Serializable types rely on some method of serialization to convert between an original state and a new one. This normally works well for situations where fields are added or removed, or additional options are opened up. Likely worth relying on serde and/or other serialization libraries, as well as opening the option of custom serialization. They can be either unidirectional (old versions upgrade) or bidirectional (if a new version is called from old code, it'll adjust for that).

Serializable types can be set up to be eager or lazy, and to be fully managed by Dexterous Developer or managed with custom tools (for example, by an adapter for a specific framework)

### Regenerated Types

These are similar to resetting types, however they require additional information rather than being based entirely on pre-provided states. These require specific tooling/implementation from an adapter or for your context, and so are less likely to be usable in framework-agnositc reloaded code.

## Functions

## Marked Reloading Functions

These are functions with a known signature & name, that are specifically marked to be reloaded. When called they will utilize the newest version of their content regardless of the calling context.

## Implicitly Reloaded Functions

Functions called by name will always be from the same version of the binary as the calling context - or an earlier one if they have not changed. This means that any functions called downstream of a marked reloading function will be the newer version, while if the same function were called outside of the scope of a marked reloading function it'll be the old version. For example:

```rust

fn unmarked_fn() {
    deeper_unmarked_fn()
}

fn deeper_unmarked_fn() {}

#[reloadable]
fn marked_reloading_fn() {
    unmarked_fn() // This will use the most recent version of both unmarked_fn() and deeper_unmarked_fn()
}

fn main() {
    while true {
        marked_reloading_fn() // This will use the most recent version of both unmarked_fn and deeper_unmarked_fn
        unmarked_fn() // This will use the original version of both unmarked_fn and deeper_unmarked_fn
    }
}
```

## Closures & Async Functions

There are 2 contexts where you might encounter code from an arbitrary version of the reloading library.

- Async functions will continue their execution at the same version they had when starting execution.
- Closures will generally point to the version available when they were created, even if called within a reloaded scope. An Async closure will execute fully within this version of the code.

This is why it can be useful to have bi-directional serializable types, to regenrate closures upon reloading, or to avoid async contexts where possible.

## Outer loops & frameworks

As a rule, hot reload is only useful if your application has some sort of outer/main loop that gets repeated regularly. If the application runs once and exits, such as in the case of the "echo" bash command, it's generally more effective to just compile and run a new version.

This means it's important to know what your loop is, and determine where you wish to inject reloadable functionality. Often frameworks will manage this loop for you - for example, bevy handles scheduling and calling systems on every frame, and axum calls your functions when requests arrive at the server.

You want to be aware of where this boundary should be for a given application. For example, if we're developing a web server with Axum, we have a few options:

- We could wrap the entire app in a reloadable context
  - This would allow easy adjusting of global settings around Axum, such as the Router
  - However, the entire server would have to "re-set" each time, and since Axum manages it's own internal state elements like the currently running request handlers or long-running websocket connections may fail - but it would allow easy adding/adjusting of routs.
- We could wrap our request handlers individually
  - This would allow us to avoid cutting-off in-progress connections, and potentially even to maintain some functionality in long-running websockets (if we place additional reloadable functions within a base websocket handler callback)
  - This would make it difficult to adjust add new routes, or adjust the signatures of our route handlers

An adapter for a specific framework can manage these options for you and allow multiple points for reloadable contexts to occur. Generally, if an adapter exists for a framework - it'll be developed with the aim of providing the most likely needed functionality while minimizing the need for adjusting your development patterns when using that framework.
