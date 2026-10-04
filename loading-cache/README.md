[![51Degrees](https://51degrees.com/img/logo.png?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-loading-cache-readme.md&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-loading-cache-readme.md&utm_term=logo)

# 51Degrees Loading Cache

A cache that loads a missing value once, however many callers ask for it at
the same time. The first caller for a key runs the load, every caller that
arrives while it runs waits for the same result, and all of them continue when
it completes. A failed load reaches every waiting caller and is not stored, so
the next caller loads again.

The cache keeps its entries in a pluggable store. This crate provides a store
in process memory, bounded by a number of entries with least recently used
eviction. A store over a platform's key-value store or cache implements the
same small trait, and may make callers in other processes wait for one load.

Caches stack by using one as the loader of another, for example a brief copy in
process memory over a shared key-value store over the source. Each cache has
its own time to live and time to idle, renews a used entry at most once per
window so an unused entry leaves the store, and never keeps a copy longer than
the copy it loaded from.

It needs no async runtime, spawns nothing and starts no threads, and it builds
and runs on native targets, `wasm32-wasip1` and `wasm32-unknown-unknown`.

This crate is part of the [51Degrees](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-loading-cache-readme.md&utm_term=introduction) Rust solution for high-performance
device detection and IP intelligence, available both on-premise from a local
data file and from the 51Degrees cloud. It is general purpose and depends on
none of the other 51Degrees crates.

## Links

- Source and issues: [github.com/51Degrees/rust](https://github.com/51Degrees/rust)
- API documentation: [docs.rs/fiftyone-loading-cache](https://docs.rs/fiftyone-loading-cache)
- About 51Degrees: [51degrees.com](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-loading-cache-readme.md&utm_term=about)

## License

Licensed under the European Union Public Licence v1.2 (EUPL-1.2). See the
[repository](https://github.com/51Degrees/rust) for the full text.
