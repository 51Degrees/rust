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

//! A store in process memory.

use std::collections::hash_map::RandomState;
use std::convert::Infallible;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use hashlink::LinkedHashMap;

use crate::clock::{self, Clock};
use crate::entry::Entry;
use crate::shards::{self, lock, Shards};
use crate::store::{Lookup, Store};

/// The default number of entries a [`MemoryStore`] holds.
pub const DEFAULT_CAPACITY: usize = 1000;

/// A [`Store`] in process memory, bounded by a number of entries.
///
/// When full it evicts the least recently used entry, and it drops an entry
/// once the `keep_for` it was written with runs out. It is the store for a
/// brief copy in front of a shared store, and it stands in for a platform
/// store in tests. It never makes callers wait.
///
/// The entries are split into shards by key, each with its own lock and an
/// equal part of the capacity, so the entry evicted is the least recently
/// used in its shard. Build it with one shard for exact order.
pub struct MemoryStore<K, V> {
    shards: Shards<LinkedHashMap<K, Kept<V>, RandomState>>,
    capacity_per_shard: usize,
    clock: Arc<dyn Clock>,
}

/// An entry and the time the store drops it.
struct Kept<V> {
    entry: Entry<V>,
    until: Option<SystemTime>,
}

impl<V> Kept<V> {
    fn is_kept(&self, now: SystemTime) -> bool {
        self.until.is_none_or(|until| now < until)
    }
}

impl<K, V> MemoryStore<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// Starts building a store.
    pub fn builder() -> MemoryStoreBuilder<K, V> {
        MemoryStoreBuilder {
            capacity: DEFAULT_CAPACITY,
            shards: shards::default_count(),
            clock: None,
            types: PhantomData,
        }
    }

    /// The number of entries held, including any past their time that have
    /// not been dropped yet.
    pub fn len(&self) -> usize {
        self.shards.iter().map(|shard| lock(shard).len()).sum()
    }

    /// True when no entries are held.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The most entries the store holds.
    pub fn capacity(&self) -> usize {
        self.capacity_per_shard * self.shards.count()
    }
}

impl<K, V> Store<K, V> for MemoryStore<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    type Reservation = Infallible;

    async fn get(&self, key: &K) -> Lookup<V, Infallible> {
        let now = self.clock.now();
        let mut map = lock(self.shards.for_key(key));
        // Moving the entry to the back marks it most recently used.
        let found = map
            .to_back(key)
            .map(|kept| kept.is_kept(now).then(|| kept.entry.clone()));
        match found {
            Some(Some(entry)) => Lookup::Hit(entry),
            Some(None) => {
                map.remove(key);
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
        let mut map = lock(self.shards.for_key(key));
        // Inserting places the entry at the back, as most recently used.
        map.insert(key.clone(), kept);
        // Drop expired entries from the least recently used end, where
        // unused entries gather, before evicting one still in use.
        while map.front().is_some_and(|(_, kept)| !kept.is_kept(now)) {
            map.pop_front();
        }
        while map.len() > self.capacity_per_shard {
            map.pop_front();
        }
    }

    async fn remove(&self, key: &K) {
        lock(self.shards.for_key(key)).remove(key);
    }
}

/// Builds a [`MemoryStore`].
pub struct MemoryStoreBuilder<K, V> {
    capacity: usize,
    shards: usize,
    clock: Option<Arc<dyn Clock>>,
    types: PhantomData<fn(K) -> V>,
}

impl<K, V> MemoryStoreBuilder<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    /// The most entries the store holds, at least one per shard. Defaults to
    /// [`DEFAULT_CAPACITY`].
    pub fn capacity(mut self, capacity: usize) -> Self {
        self.capacity = capacity;
        self
    }

    /// How many parts the entries are split into, each with its own lock.
    /// Defaults to the number of processors.
    pub fn shards(mut self, shards: usize) -> Self {
        self.shards = shards;
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
    pub fn build(self) -> MemoryStore<K, V> {
        let shards = Shards::new(self.shards, || {
            LinkedHashMap::with_hasher(RandomState::new())
        });
        // Rounded up so the total is never below the capacity asked for.
        let capacity_per_shard = self.capacity.max(1).div_ceil(shards.count());
        MemoryStore {
            shards,
            capacity_per_shard,
            clock: clock::given_or_system(self.clock),
        }
    }
}
