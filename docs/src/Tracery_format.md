# Tracery Asset Format

When using the `serde` feature - Tracery Grammar implements the serde serialize/deserialize traits. This allows you to use many different formats to store the grammars.

In addition, with the `asset` feature, we use [Bevy Common Assets](https://github.com/NiklasEi/bevy_common_assets) to implement a multi-file-type asset plugin (found under `bevy_generative_grammars::tracery::tracery_asset::TraceryAssetPlugin`) for bevy. You can enable any of the formats supported by *Bevy Common Assets* using their matching trait - for example the `json` trait for JSON files.

When serializing/deserializing formats, we assume the following structure:

```typescript
{
    "rules": {
        [key: string]: string[]
    },
    "starting_point"?: string
}
```

The `rules` structure matches the structure of a tracery grammar by default, and the optional `starting_point` provides an alternative default starting point (otherwise, we use `origin`).
