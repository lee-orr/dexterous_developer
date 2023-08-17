# Simple Tracery Example

Now we want to actually remember who we're talking to... so let's adjust our grammar.

```json
{
    // all we did was set up a friend variable and start using it instead of name
    "origin": ["[friend:#name#]Hello #friend#!", "[friend:#name#]#friend#, you made it!"],
    // and we added a second sentence that uses it too.
    "second_sentence": ["I missed you #friend#", "#friend, it's been too long"],
    "name": ["Jane", "Alice", "Bob", "Roberto"]
}
```

And we update our rules.

```rust
const RULES: &[(&str, &[&str])] = &[
    (
        "origin",
        &["[friend:#name#]Hello #friend#!", "[friend:#name#]#friend#, you made it!"],
    ),
    (
        "second_sentence", &["I missed you #friend#", "#friend, it's been too long"],
    )
    (
        "name",
        &["Jane", "Alice", "Bob", "Roberto"],
    ),
]
```

Now, the `StringGenerator` we used can't retain state. That means we can't use this kind of more complex grammar with it, since it won't remember our new rules. For that, we can use the `StatefulStringGenerator` instead:

```rust
fn main() {
    // We can use an existing grammar if we have one, but in this case we are just creating the generator directly.
    let mut generator = StatefulStringGenerator::new(RULE, None);

    // Then we create the rng, like before
    let mut rng = |_| {
        0
    };

    // Now we generate our story - it should print out:
    // "Hello Jane", just like before.
    // Note that we don't need to provide the grammar.
    match generator.generate(&mut rng) {
        Some(result) => {
            println!("{result}");
        },
        None => {
            eprintln!("There was an error...");
        }
    }

    // But then we can go to the second sentence, which prints out:
    // "I missed you Jane"
    match generator.generate_at(&mut rng, "second_sentence") {
        Some(result) => {
            println!("{result}");
        },
        None => {
            eprintln!("There was an error...");
        }
    }

    // Or even use it in an expandable prompt like this:
    let story = "Agnus called me \"#friend#\"... we don't even look that similar!".to_string();
    let result = generator.expand_from(&story, &mut rng);
    // Which prints out:
    // "Agnus called me "Jane"... we don't even look that similar!"
    println!("{result}");
}
```
