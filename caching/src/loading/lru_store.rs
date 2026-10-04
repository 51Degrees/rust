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

//! A store in process memory over the sharded LRU cache.

use std::convert::Infallible;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use super::clock::{self, Clock};
use super::entry::Entry;
use super::store::{Lookup, Store};
use crate::cache::{Cache, PutCache};
use crate::config::CacheBuilder;
use crate::lru::LruCache;

/// A [`Store`] in process memory, the sharded [`LruCache`] with lifetimes.
///
/// When full it evicts the least recently used entry in the entry's shard,
/// and it drops an entry once the `keep_for` it was written with runs out. It
/// is the store of an [`LruLoadingCache`](crate::LruLoadingCache), and stands
/// in for a platform store in tests. It never makes callers wait.
pub struct LruStore<K, V>
where
    K: Hash + Eq + Send + Sync,
    V: Clone + Send + Sync,
{
    cache: LruCache<K, Kept<V>>,
    clock: Arc<dyn Clock>,
}

/// An entry and the time the store drops it.
#[derive(Clone)]
struct Kept<V> {
    entry: Entry<V>,
    until: Option<SystemTime>,
}

impl<K, V> LruStore<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// Starts building a store, with the [`CacheBuilder`] defaults.
    pub fn builder() -> LruStoreBuilder<K, V> {
        LruStoreBuilder {
            cache: CacheBuilder::new(),
            clock: None,
            types: PhantomData,
        }
    }

    /// The number of entries held, including any past their time that have
    /// not been read since.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// True when no entries are held.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// The most entries the store holds.
    pub fn capacity(&self) -> usize {
        self.cache.capacity()
    }
}

impl<K, V> Store<K, V> for LruStore<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    type Reservation = Infallible;

    async fn get(&self, key: &K) -> Lookup<V, Infallible> {
        let now = self.clock.now();
        // Reading marks the entry most recently used.
        match self.cache.get(key) {
            Some(kept) if kept.until.is_none_or(|until| now < until) => Lookup::Hit(kept.entry),
            Some(_) => {
                self.cache.remove(key);
                Lookup::Miss
            }
            None => Lookup::Miss,
        }
    }

    async fn put(&self, key: &K, entry: &Entry<V>, keep_for: Option<Duration>) {
        let now = self.clock.now();
        let kept = Kept {
            entry: entry.clone(),
            until: keep_for.and_then(|keep_for| now.checked_add(keep_for)),
        };
        self.cache.put(key.clone(), kept);
    }

    async fn remove(&self, key: &K) {
        self.cache.remove(key);
    }
}

/// Builds an [`LruStore`].
pub struct LruStoreBuilder<K, V> {
    cache: CacheBuilder,
    clock: Option<Arc<dyn Clock>>,
    types: PhantomData<fn(K) -> V>,
}

impl<K, V> LruStoreBuilder<K, V>
where
    K: Hash + Eq + Clone + Send + Sync,
    V: Clone + Send + Sync,
{
    /// The most entries the store holds. Defaults to
    /// [`DEFAULT_SIZE`](crate::DEFAULT_SIZE).
    pub fn size(mut self, size: usize) -> Self {
        self.cache = self.cache.size(size);
        self
    }

    /// How many shards the entries are split into, each with its own lock.
    /// Defaults to the number of processors.
    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.cache = self.cache.concurrency(concurrency);
        self
    }

    /// The clock that decides when entries are dropped. Defaults to the
    /// system clock, which `wasm32-unknown-unknown` does not have.
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = Some(clock);
        self
    }

    /// Builds the store.
    ///
    /// # Panics
    ///
    /// On `wasm32-unknown-unknown` when no [clock](Self::clock) was given.
    pub fn build(self) -> LruStore<K, V> {
        LruStore {
            cache: self.cache.build(),
            clock: clock::given_or_system(self.clock),
        }
    }
}
