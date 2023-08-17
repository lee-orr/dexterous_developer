# Random Generation Traits

This crate exposes a trait for the random generation process it expects - the `GrammarRandomNumberGenerator` trait:

```rust
pub trait GrammarRandomNumberGenerator {
    /// This function provides a random number between 0 and len
    fn get_number(&mut self, len: usize) -> usize;
}
```

We provide a few built in implementations for it:

- `usize` - we have set it up so usize implements the trait, always returning it's value.
- `FnMut(usize) -> usize` - if the length is greater then zero, it will call the closure/FnMut and return it's value

In addition, we provide wrapper components for use with 2 different random number generation crates - `rand` and `bevy_turborand` - hidden behind feature flags.

If you wish to use the `rand` crate, enable the `rand` feature and wrap any type implementing `rand::Rng` using either:

- `Rand::new(&mut rng)` - this provides a wrapper using the existing reference, and bound to it's lifetime.
- `RandOwned::new(rng)` - this provides a wrapper that takes over the existing type, and ownes it from this point forward.

If you wish to use the `bevy_turborand` crate, enable the `turborand` feature and wrap any type implementing `bevy_turborand::TurboRand` using either:

- `TurboRand::new(&mut rng)` - this provides a wrapper using the existing reference, and bound to it's lifetime.
- `TurboRandOwned::new(rng)` - this provides a wrapper that takes over the existing type, and ownes it from this point forward.
