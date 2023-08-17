# Introduction

This library provides the capacity to use generative grammars within bevy.

At the moment, it provides two things:

- an set of traits that an be used to build your own grammars (in `bevy_generative_grammars::generator::*`)
- an implementation of the [tracery generative grammar](https://github.com/galaxykate/tracery) - in both a stateful generator and a stateless one (in `bevy_generative_grammars::tracery::*`). These are able to act as bevy resources or components, and use the bevy hashmap implementation internally.

We also have [API Docs](https://lee-orr.github.io/bevy-generative-grammars/doc/bevy_generative_grammars/index.html)
