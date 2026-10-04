/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

//! [![51Degrees](https://51degrees.com/img/logo.png?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fiftyone-loading-cache-lib.rs&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fiftyone-loading-cache-lib.rs&utm_term=logo)
//!
//! # 51Degrees loading cache
//!
//! A cache that loads a missing value once, however many callers ask for it
//! at the same time. The first caller for a key runs the load, every caller
//! that arrives while it runs waits for the same result, and all of them
//! continue when it completes. A failed load reaches every waiting caller and
//! is not stored, so the next caller loads again.
//!
//! The crate depends on no runtime, spawns nothing and starts no threads. It
//! builds and runs on native targets, on `wasm32-wasip1` and on
//! `wasm32-unknown-unknown`, so one cache serves a long-lived native server
//! and a long-lived WebAssembly instance alike.
//!
//! ## The pieces
//!
//! - [`LoadingCache`] does all the work. It is generic over a [`Store`] and a
//!   [`Loader`].
//! - A [`Store`] keeps entries. [`MemoryStore`] keeps them in process
//!   memory. A store over a platform's key-value store or cache is written
//!   against the same trait.
//! - A [`Loader`] produces a value the store does not hold. The source is a
//!   loader, [`from_fn`] makes one from a function, and every
//!   [`LoadingCache`] is one.
//! - A [`Clock`] gives the time. The system clock is the default, except on
//!   `wasm32-unknown-unknown`, which has none, so a host there supplies one.
//!
//! ## Layers
//!
//! Caches stack by using one as the loader of another. A brief copy in
//! process memory, over a shared key-value store, over the source:
//!
//! ```text
//! LoadingCache(MemoryStore, time to live 5 s)
//!   loads from LoadingCache(key-value store, time to live 1 day, idle 1 hour)
//!     loads from the source
//! ```
//!
//! A read looks in memory first. On a miss it asks the cache below, which
//! looks in the key-value store, and only a miss there reaches the source.
//! Each cache collapses its own concurrent misses, so many callers missing
//! in memory make one call to the cache below, and many processes missing
//! in a store that can make callers wait make one call to the source. A
//! value changed or removed in the shared store is seen once the memory
//! copy's short life ends.
//!
//! A value carries the time it was written and the time it stops being
//! usable, and a cache never keeps a copy longer than the copy it loaded
//! from, so no copy in a stack outlives the copy below it.
//!
//! ## What a store does and what the cache does
//!
//! A store keeps each entry for the time the cache tells it when writing,
//! and may drop entries sooner to make room. A store may also make callers
//! in other processes wait for one load, by answering
//! [`Lookup::Reserved`].
//!
//! The cache does the rest.
//!
//! - It allows one load per key at a time in its process.
//! - It decides each copy's lifetime from its time to live, its time to
//!   idle and the copy it was loaded from.
//! - It renews a used copy at most once per renewal window, so a copy left
//!   unused for the idle time leaves the store.
//! - It checks every entry it reads is still fresh, so a store that drops
//!   entries late, or never, still gives correct results.
//!
//! ## Example
//!
//! ```
//! use std::time::Duration;
//! use fiftyone_loading_cache::{Loaded, Loader, LoadingCache, MemoryStore};
//!
//! /// Fetches a page from the origin.
//! struct Origin;
//!
//! impl Loader<String, String> for Origin {
//!     type Error = String;
//!
//!     async fn load(&self, url: &String) -> Result<Loaded<String>, String> {
//!         Ok(Loaded::new(format!("page at {url}")))
//!     }
//! }
//!
//! /// The shared copies. A memory store stands in for a platform store.
//! type Shared = LoadingCache<String, String, MemoryStore<String, String>, Origin>;
//!
//! /// A brief copy in memory over the shared copies.
//! type Pages = LoadingCache<String, String, MemoryStore<String, String>, Shared>;
//!
//! fn pages() -> Pages {
//!     let shared = LoadingCache::builder(MemoryStore::builder().build(), Origin)
//!         .time_to_live(Duration::from_secs(24 * 60 * 60))
//!         .time_to_idle(Duration::from_secs(60 * 60))
//!         .build();
//!     LoadingCache::builder(MemoryStore::builder().capacity(100).build(), shared)
//!         .time_to_live(Duration::from_secs(5))
//!         .build()
//! }
//!
//! async fn page(pages: &Pages, url: &String) -> Result<String, String> {
//!     pages.get(url).await
//! }
//! # let _ = (pages(), page);
//! ```

#![warn(missing_docs)]

mod cache;
mod clock;
mod entry;
mod flight;
mod loader;
mod memory;
mod shards;
mod store;

pub use cache::{LoadingCache, LoadingCacheBuilder};
pub use clock::Clock;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub use clock::SystemClock;
pub use entry::{Entry, Loaded};
pub use loader::{from_fn, FnLoader, Loader};
pub use memory::{MemoryStore, MemoryStoreBuilder, DEFAULT_CAPACITY};
pub use store::{Lookup, Store};
