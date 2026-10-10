[![51Degrees](https://51degrees.com/img/logo.png?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-caching-readme.md&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-caching-readme.md&utm_term=logo)

# 51Degrees Caching

Sharded-Least Recently Used (LRU) cache trait and default implementation for the 51Degrees pipeline.

It also holds the loading cache, which loads a missing value once however
many callers ask for it at the same time. The first caller for a key runs the
load, every caller that arrives meanwhile waits for the same result, and a
failed load reaches every waiting caller without being stored. The loading
cache keeps its entries in a pluggable store, from the least recently used
cache in process memory to a platform's key-value store, and caches stack by
using one as the loader of another. A host that can run work on its own can
give the cache a spawner, so each load runs as a task of its own and a
dropped caller neither stops a load nor starts another. The names follow the
caching packages of the other languages, being `LoadingCache`,
`LruLoadingCache`, `LoadingCacheBuilder` and `ValueLoader`.

The crate builds for native targets and `wasm32-wasip1`, and for
`wasm32-unknown-unknown` with default features off. The `pipeline` feature,
on by default, brings `DataKeyedCache` and the dependency on
`fiftyone-pipeline-core` it needs.

This crate is part of the [51Degrees](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-caching-readme.md&utm_term=introduction) Rust solution for high-performance
device detection and IP intelligence, available both on-premise from a local
data file and from the 51Degrees cloud.

## Links

- Source and issues: [github.com/51Degrees/rust](https://github.com/51Degrees/rust)
- API documentation: [docs.rs/fiftyone-caching](https://docs.rs/fiftyone-caching)
- About 51Degrees: [51degrees.com](https://51degrees.com/?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-caching-readme.md&utm_term=about)
- Data files and pricing: [51degrees.com/pricing](https://51degrees.com/pricing?utm_source=github&utm_medium=readme&utm_campaign=rust&utm_content=fiftyone-caching-readme.md&utm_term=pricing)

## License

Licensed under the European Union Public Licence v1.2 (EUPL-1.2). See the
[repository](https://github.com/51Degrees/rust) for the full text.
