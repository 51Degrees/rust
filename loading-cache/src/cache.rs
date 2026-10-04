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

//! The loading cache.

use std::future::Future;
use std::hash::Hash;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crate::clock::{self, Clock};
use crate::entry::{earliest, Entry, Loaded};
use crate::flight::{Flights, Lead, Outcome, Role};
use crate::loader::Loader;
use crate::shards;
use crate::store::{Lookup, Store};

/// A cache that loads a missing value once, however many callers ask for it
/// at the same time.
///
/// The first caller for a key that is not in the [`Store`] runs the
/// [`Loader`]. Every caller that asks for the key meanwhile waits for that
/// load, and all of them get its value or its error. A failed load is not
/// stored, so the next caller loads again. If the caller running the load is
/// dropped, a waiting caller takes over.
///
/// A `LoadingCache` is itself a [`Loader`], so a cache over a small store in
/// memory can load from a cache over a shared store, which loads from the
/// source. Each cache has its own lifetimes, and a cache never keeps a copy
/// longer than the copy it loaded it from.
///
/// # Example
///
/// ```
/// use std::time::Duration;
/// use fiftyone_loading_cache::{from_fn, LoadingCache, MemoryStore};
///
/// // The shared store would be a platform's key-value store. A memory store
/// // stands in for it here.
/// let shared = LoadingCache::builder(
///     MemoryStore::builder().capacity(10_000).build(),
///     from_fn(|url: String| async move { Ok::<_, String>(format!("page {url}")) }),
/// )
/// .time_to_live(Duration::from_secs(24 * 60 * 60))
/// .time_to_idle(Duration::from_secs(60 * 60))
/// .build();
///
/// // A brief copy in process memory, loading from the shared cache.
/// let cache = LoadingCache::builder(MemoryStore::builder().capacity(1000).build(), shared)
///     .time_to_live(Duration::from_secs(5))
///     .build();
/// # let _ = &cache;
/// ```
pub struct LoadingCache<K, V, S, L>
where
    L: Loader<K, V>,
{
    store: S,
    loader: L,
    lifetimes: Lifetimes,
    clock: Arc<dyn Clock>,
    flights: Flights<K, Result<Loaded<V>, L::Error>>,
}

impl<K, V, S, L> LoadingCache<K, V, S, L>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: Store<K, V>,
    L: Loader<K, V>,
{
    /// Starts building a cache that keeps its entries in `store` and loads
    /// missing values with `loader`.
    pub fn builder(store: S, loader: L) -> LoadingCacheBuilder<K, V, S, L> {
        LoadingCacheBuilder {
            store,
            loader,
            time_to_live: None,
            time_to_idle: None,
            renewal_window: None,
            clock: None,
            shards: shards::default_count(),
            types: PhantomData,
        }
    }

    /// The value for `key`, from the store or loaded.
    pub async fn get(&self, key: &K) -> Result<V, L::Error> {
        self.get_loaded(key).await.map(|loaded| loaded.value)
    }

    /// The value for `key` with the time it was written and the time this
    /// cache's copy stops being usable. A cache above uses these times to
    /// keep its own copy no longer.
    pub async fn get_loaded(&self, key: &K) -> Result<Loaded<V>, L::Error> {
        loop {
            match self.flights.join_or_lead(key) {
                Role::Wait(wait) => match wait.await {
                    Outcome::Done(result) => return result,
                    Outcome::Abandoned => continue,
                },
                Role::Lead(lead) => return self.lead(key, lead).await,
            }
        }
    }

    /// Removes `key` from this cache's store. Caches below keep their copies.
    pub async fn remove(&self, key: &K) {
        self.store.remove(key).await;
    }

    /// The store this cache keeps its entries in.
    pub fn store(&self) -> &S {
        &self.store
    }

    /// The work of the one caller leading the load of `key`. It publishes
    /// the result to the waiting callers as soon as it has one, then writes
    /// to the store.
    async fn lead(
        &self,
        key: &K,
        lead: Lead<'_, K, Result<Loaded<V>, L::Error>>,
    ) -> Result<Loaded<V>, L::Error> {
        let now = self.clock.now();
        let reservation = match self.store.get(key).await {
            Lookup::Hit(entry) if self.lifetimes.is_fresh(&entry, now) => {
                let renewed = self.lifetimes.renewal(&entry, now);
                let served = self.lifetimes.served(renewed.as_ref().unwrap_or(&entry));
                lead.publish(Ok(served.clone()));
                if let Some(renewed) = renewed {
                    self.write(key, None, &renewed, now).await;
                }
                return Ok(served);
            }
            Lookup::Hit(_) | Lookup::Miss => None,
            Lookup::Reserved(reservation) => Some(reservation),
        };
        match self.loader.load(key).await {
            Ok(loaded) => {
                let now = self.clock.now();
                let entry = self.lifetimes.copy(loaded, now);
                let served = self.lifetimes.served(&entry);
                lead.publish(Ok(served.clone()));
                self.write(key, reservation, &entry, now).await;
                Ok(served)
            }
            Err(error) => {
                // Release callers waiting in the store before the ones
                // waiting here.
                drop(reservation);
                lead.publish(Err(error.clone()));
                Err(error)
            }
        }
    }

    /// Writes `entry` to the store, filling `reservation` when there is one.
    /// An entry already past its end at `now` is not written.
    async fn write(
        &self,
        key: &K,
        reservation: Option<S::Reservation>,
        entry: &Entry<V>,
        now: SystemTime,
    ) {
        let Some(keep_for) = self.lifetimes.keep_for(entry, now) else {
            return;
        };
        match reservation {
            Some(reservation) => self.store.fill(key, reservation, entry, keep_for).await,
            None => self.store.put(key, entry, keep_for).await,
        }
    }
}

impl<K, V, S, L> Loader<K, V> for LoadingCache<K, V, S, L>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: Store<K, V>,
    L: Loader<K, V>,
{
    type Error = L::Error;

    fn load(&self, key: &K) -> impl Future<Output = Result<Loaded<V>, Self::Error>> {
        self.get_loaded(key)
    }
}

/// Builds a [`LoadingCache`].
pub struct LoadingCacheBuilder<K, V, S, L> {
    store: S,
    loader: L,
    time_to_live: Option<Duration>,
    time_to_idle: Option<Duration>,
    renewal_window: Option<Duration>,
    clock: Option<Arc<dyn Clock>>,
    shards: usize,
    types: PhantomData<fn(K) -> V>,
}

impl<K, V, S, L> LoadingCacheBuilder<K, V, S, L>
where
    K: Hash + Eq + Clone,
    V: Clone,
    S: Store<K, V>,
    L: Loader<K, V>,
{
    /// How long a copy is used, from when this cache writes it. Without one
    /// a copy lasts as long as the copy it was loaded from.
    pub fn time_to_live(mut self, time_to_live: Duration) -> Self {
        self.time_to_live = Some(time_to_live);
        self
    }

    /// How long a copy lasts unused. Using a copy renews it, at most once
    /// per [renewal window](Self::renewal_window), and the store is told to
    /// keep it no longer than this, so a copy left unused leaves the store.
    pub fn time_to_idle(mut self, time_to_idle: Duration) -> Self {
        self.time_to_idle = Some(time_to_idle);
        self
    }

    /// The least time between two renewals of one copy, so a busy key does
    /// not write to the store on every use. A copy can therefore leave up to
    /// this much sooner than [`time_to_idle`](Self::time_to_idle) after its
    /// last use. Defaults to a quarter of the idle time, and may be at most
    /// half of it.
    pub fn renewal_window(mut self, renewal_window: Duration) -> Self {
        self.renewal_window = Some(renewal_window);
        self
    }

    /// The clock the cache reads. Defaults to the system clock, which
    /// `wasm32-unknown-unknown` does not have.
    pub fn clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = Some(clock);
        self
    }

    /// How many parts the record of loads in progress is split into, so
    /// callers loading different keys rarely wait for the same lock.
    /// Defaults to the number of processors.
    pub fn shards(mut self, shards: usize) -> Self {
        self.shards = shards;
        self
    }

    /// Builds the cache.
    ///
    /// # Panics
    ///
    /// If the renewal window is more than half the idle time, or on
    /// `wasm32-unknown-unknown` when no [clock](Self::clock) was given.
    pub fn build(self) -> LoadingCache<K, V, S, L> {
        let renewal_window = match (self.time_to_idle, self.renewal_window) {
            (Some(idle), Some(window)) => {
                assert!(
                    window <= idle / 2,
                    "the renewal window must be at most half the idle time"
                );
                window
            }
            (Some(idle), None) => idle / 4,
            (None, window) => window.unwrap_or_default(),
        };
        LoadingCache {
            store: self.store,
            loader: self.loader,
            lifetimes: Lifetimes {
                time_to_live: self.time_to_live,
                time_to_idle: self.time_to_idle,
                renewal_window,
            },
            clock: clock::given_or_system(self.clock),
            flights: Flights::new(self.shards),
        }
    }
}

/// How long one cache's copies may be used.
#[derive(Debug, Clone, Copy)]
struct Lifetimes {
    time_to_live: Option<Duration>,
    time_to_idle: Option<Duration>,
    renewal_window: Duration,
}

impl Lifetimes {
    /// Whether `entry` may be used at `now`.
    fn is_fresh<V>(&self, entry: &Entry<V>, now: SystemTime) -> bool {
        let before = |end: SystemTime| now < end;
        entry.expires.is_none_or(before) && self.idle_end(entry).is_none_or(before)
    }

    /// When `entry` lapses unless it is used and renewed.
    fn idle_end<V>(&self, entry: &Entry<V>) -> Option<SystemTime> {
        self.time_to_idle
            .and_then(|idle| entry.renewed.checked_add(idle))
    }

    /// This cache's copy of a loaded value, written at `now`. It ends at
    /// this cache's time to live or the loaded value's own end, whichever is
    /// sooner.
    fn copy<V>(&self, loaded: Loaded<V>, now: SystemTime) -> Entry<V> {
        let own_end = self.time_to_live.and_then(|ttl| now.checked_add(ttl));
        Entry {
            value: loaded.value,
            written: loaded.written.unwrap_or(now),
            expires: earliest(own_end, loaded.expires),
            renewed: now,
        }
    }

    /// The copy of `entry` renewed at `now`, when the use at `now` is due to
    /// renew it and doing so lets it last longer.
    fn renewal<V: Clone>(&self, entry: &Entry<V>, now: SystemTime) -> Option<Entry<V>> {
        let idle_end = self.idle_end(entry)?;
        let due = entry
            .renewed
            .checked_add(self.renewal_window)
            .is_some_and(|due| now >= due);
        let extends = entry.expires.is_none_or(|end| end > idle_end);
        (due && extends).then(|| Entry {
            renewed: now,
            ..entry.clone()
        })
    }

    /// How `entry` is given to callers, including a cache above. Its end is
    /// brought forward to one renewal window before its idle end, so a cache
    /// above comes back for the value while this cache can still renew it.
    fn served<V: Clone>(&self, entry: &Entry<V>) -> Loaded<V> {
        let renew_by = self.time_to_idle.and_then(|idle| {
            entry
                .renewed
                .checked_add(idle.saturating_sub(self.renewal_window))
        });
        Loaded {
            value: entry.value.clone(),
            written: Some(entry.written),
            expires: earliest(entry.expires, renew_by),
        }
    }

    /// How long the store should keep `entry`, written at `now`. `None`
    /// means the entry is already past its end and is not written. The
    /// store keeps it until its end, or for the idle time, whichever is
    /// sooner, since a renewal writes it again.
    fn keep_for<V>(&self, entry: &Entry<V>, now: SystemTime) -> Option<Option<Duration>> {
        let until_end = match entry.expires {
            Some(end) => match end.duration_since(now) {
                Ok(left) if !left.is_zero() => Some(left),
                _ => return None,
            },
            None => None,
        };
        Some(match (until_end, self.time_to_idle) {
            (Some(left), Some(idle)) => Some(left.min(idle)),
            (left, idle) => left.or(idle),
        })
    }
}
