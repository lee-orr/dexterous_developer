# Simple Tracery Example

In this case, we're going to implement an example that processes the following tracery grammar:

```json
{
    "origin": ["Hello #name#!", "#name#, you made it!"],
    "name": ["Jane", "Alice", "Bob", "Roberto"]
}
```

First off, we need to set up the rules - in this case we'll make them a constant:

```rust
const RULES: &[(&str, &[&str])] = &[
    (
        "origin",
        &["Hello #name#!", "#name#, you made it!"],
    ),
    (
        "name",
        &["Jane", "Alice", "Bob", "Roberto"],
    ),
]
```

Next, in our main function:

```rust
fn main() {
    // We need to load in our grammar - this stores an immutable copy of our ruleset
    let grammar = TraceryGrammar::new(RULES, None);

    // Next, we need to setup our random generation function.
    // Normally, you'll want to use rand or bevy_turborand for this.
    // But we're just going to hard code it, so we have consisten results.
    let mut rng = 0;

    // Now we generate our story - it should print out:
    // "Hello Jane"
    match StringGenerator:generate(&grammar, &mut rng) {
        Some(result) => {
            println!("{result}");
        },
        None => {
            eprintln!("There was an error...");
        }
    } 
}
```

If you want more information on random number generation - take a look at our [Random Number Generation docs](./random_generation_traits.md).

Now, what if we just want a name?
In that case, we can adjust the string generation line from:

```rust
    match StringGenerator:generate(&grammar, &mut rng) {
```

to:

```rust
    match StringGenerator:generate_at(&"name".to_string(), &grammar, &mut rng) {
```

Which will print out `Jane`

Or - we might want to provide it with some arbitrary content to expand. In this case we'd replace the entire match statement with something like this:

```rust
    let story = "Agnus greeted me, saying \"#origin#\"".to_string();
    let result = StringGenerator::expand_from(&story, &grammar, &mut rng);
    println!("{result}");
```

Notice that using "expand_from" doesn't require an option, since it will always at least return the initial input, if it can't expand it further.
