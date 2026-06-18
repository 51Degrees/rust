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

//! [![51Degrees](https://51degrees.com/img/logo.png?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fiftyone-caching-lib.rs&utm_term=logo "Data rewards the curious")](https://51degrees.com/?utm_source=docs.rs&utm_medium=docs&utm_campaign=rust&utm_content=fiftyone-caching-lib.rs&utm_term=logo)
//!
//! # 51Degrees caching
//!
//! A sharded least-recently-used cache for the 51Degrees pipeline. It
//! implements the custom cache described in the
//! [caching specification](https://github.com/51Degrees/specifications/blob/main/pipeline-specification/features/caching.md)
//! and exists chiefly to speed up the cloud request engine when many requests
//! share the same evidence.
//!
//! ## Why sharded
//!
//! A general-purpose cache was found too slow on the pipeline's hot path. This
//! cache is kept simple and predictable instead. The entries are split across a
//! fixed number of shards, each a [`std::sync::Mutex`] around an
//! insertion-ordered map, so concurrent requests contend on a lock only when
//! they hash to the same shard. Memory use is bounded because each shard evicts
//! its least-recently-used entry once full.
//!
//! ## The pieces
//!
//! - [`Cache`] and [`PutCache`] are the small read and write trait surfaces, so
//!   an engine can depend on the abstraction rather than the concrete cache.
//! - [`LruCache`] is the default sharded LRU implementation. It is generic over
//!   a key `K` and a value `V`, both `Send + Sync`, with `V: Clone`. It is
//!   never generic over `dyn ElementData` because element data is not `Sync`.
//!   Engines cache their own concrete `Send + Sync + Clone` data, typically an
//!   [`std::sync::Arc`] around an aspect-data struct, or the cloud JSON.
//! - [`CacheBuilder`] applies the two tunables from the specification: the
//!   total `size` (default 1000) and the `concurrency`, the number of shards
//!   (default the CPU count).
//! - [`DataKeyedCache`] wraps an [`LruCache`] keyed by
//!   [`fiftyone_pipeline_core::DataKey`]. An engine hands it a flow data and an
//!   [`fiftyone_pipeline_core::EvidenceKeyFilter`]; it derives a deterministic,
//!   case-insensitive key from the relevant evidence, so equivalent requests
//!   share an entry.
//!
//! ## A minimal cache
//!
//! ```
//! use fiftyone_caching::{Cache, LruCache, PutCache};
//!
//! let cache: LruCache<String, u32> = LruCache::with_defaults();
//! cache.put("query.user-agent=abc".to_owned(), 42);
//! assert_eq!(cache.get(&"query.user-agent=abc".to_owned()), Some(42));
//! assert_eq!(cache.get(&"missing".to_owned()), None);
//! ```

#![warn(missing_docs)]

mod cache;
mod config;
mod data_keyed;
mod lru;

pub use cache::{Cache, PutCache};
pub use config::{default_concurrency, CacheBuilder, DEFAULT_SIZE};
pub use data_keyed::DataKeyedCache;
pub use lru::LruCache;
