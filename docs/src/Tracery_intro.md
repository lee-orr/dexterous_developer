# Introduction to Tracery

For a quick tutorial on Tracery, look at the [Tracery interactive tutorial](https://web.archive.org/web/20230307220020/http://www.crystalcodepalace.com/traceryTut.html) - courtesy of the wayback machine.

Once you understand it, there are a few modifications to the basic syntax that apply in this project:

- First, you are not limited to storing your grammar in JSON - [look here for more info](./Tracery_format.md)
- Second, the library currently doesn't support any of the modifiers supported by the original JS implementation.
- Lastly, in addition to being able to save data that get's worked out in the moment, like so `[variable:some text to process]`, you can save data in way that will be processed at a later point - like so `[variable|some text to process]`. Essentially, this allows you to create re-directions that go do different rules based on remembered context. This is particularly useful since you can use a stateful generator to continue generation from a pre-existing state. I recommend looking at the example asset in `/assets/story.json` to see a complex version supporting all the syntax we support.
