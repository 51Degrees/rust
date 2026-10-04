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

//! One load per key at a time, with many requests interleaved on one
//! thread. Every check runs once over the memory store and once over a
//! store that makes callers wait, so the one cache is shown to work with
//! both kinds of store.

mod common;

use common::*;
use fiftyone_loading_cache::{LoadingCache, Store};

const CALLERS: usize = 8;

fn cache<S: Store<u32, String>>(
    store: S,
    source: &Source,
    clock: &TestClock,
) -> LoadingCache<u32, String, S, Source> {
    LoadingCache::builder(store, source.clone())
        .clock(clock.shared())
        .build()
}

/// Every caller is polled once before the load may finish, so all of them
/// miss together. One load means they were collapsed.
fn concurrent_misses_share_one_load<S: Store<u32, String>>(make: fn(&TestClock) -> S) {
    let clock = TestClock::new();
    let source = Source::gated();
    let cache = cache(make(&clock), &source, &clock);

    let callers = (0..CALLERS).map(|_| boxed(cache.get(&1))).collect();
    let results = run_rounds(callers, |round, _| {
        if round == 1 {
            source.gate.open();
        }
    });

    assert_eq!(source.loads(), 1);
    for result in results {
        assert_eq!(result, Some(Ok("1 from load 1".to_owned())));
    }
}

/// A failed load reaches every waiting caller, is not stored, and the next
/// caller loads again.
fn a_failed_load_reaches_every_waiter_and_is_not_kept<S: Store<u32, String>>(
    make: fn(&TestClock) -> S,
) {
    let clock = TestClock::new();
    let source = Source::gated();
    source.fail(true);
    let cache = cache(make(&clock), &source, &clock);

    let callers = (0..CALLERS).map(|_| boxed(cache.get(&1))).collect();
    let results = run_rounds(callers, |round, _| {
        if round == 1 {
            source.gate.open();
        }
    });

    assert_eq!(source.loads(), 1);
    for result in results {
        assert_eq!(result, Some(Err("load 1 of 1 failed".to_owned())));
    }

    source.fail(false);
    assert_eq!(block_on(cache.get(&1)), Ok("1 from load 2".to_owned()));
    assert_eq!(source.loads(), 2);
}

/// When the caller running the load is dropped, a waiting caller loads
/// instead, and every remaining caller gets that value.
fn a_cancelled_load_passes_to_a_waiting_caller<S: Store<u32, String>>(make: fn(&TestClock) -> S) {
    let clock = TestClock::new();
    let source = Source::gated();
    let cache = cache(make(&clock), &source, &clock);

    let callers = (0..3).map(|_| boxed(cache.get(&1))).collect();
    let results = run_rounds(callers, |round, callers| {
        if round == 1 {
            callers[0] = None;
            source.gate.open();
        }
    });

    assert_eq!(source.loads(), 2);
    assert_eq!(results[0], None);
    assert_eq!(results[1], Some(Ok("1 from load 2".to_owned())));
    assert_eq!(results[2], Some(Ok("1 from load 2".to_owned())));
}

/// Callers for different keys do not wait for each other.
fn different_keys_load_separately<S: Store<u32, String>>(make: fn(&TestClock) -> S) {
    let clock = TestClock::new();
    let source = Source::new();
    let cache = cache(make(&clock), &source, &clock);

    let keys = [1, 2, 3];
    let results = run_all(keys.iter().map(|key| boxed(cache.get(key))).collect());

    assert_eq!(source.loads(), 3);
    let mut results: Vec<String> = results.into_iter().map(Result::unwrap).collect();
    results.sort();
    assert_eq!(
        results,
        ["1 from load 1", "2 from load 2", "3 from load 3"].map(String::from)
    );
}

/// A value once loaded is served from the store.
fn a_stored_value_is_not_loaded_again<S: Store<u32, String>>(make: fn(&TestClock) -> S) {
    let clock = TestClock::new();
    let source = Source::new();
    let cache = cache(make(&clock), &source, &clock);

    for _ in 0..3 {
        assert_eq!(block_on(cache.get(&1)), Ok("1 from load 1".to_owned()));
    }
    assert_eq!(source.loads(), 1);
}

fn memory_store(clock: &TestClock) -> fiftyone_loading_cache::MemoryStore<u32, String> {
    memory(clock, 100)
}

/// Generates one test per store for a check.
macro_rules! for_each_store {
    ($check:ident) => {
        mod $check {
            use super::*;

            #[test]
            fn memory_store() {
                $check(super::memory_store);
            }

            #[test]
            fn waiting_store() {
                $check(WaitingStore::new);
            }
        }
    };
}

for_each_store!(concurrent_misses_share_one_load);
for_each_store!(a_failed_load_reaches_every_waiter_and_is_not_kept);
for_each_store!(a_cancelled_load_passes_to_a_waiting_caller);
for_each_store!(different_keys_load_separately);
for_each_store!(a_stored_value_is_not_loaded_again);
