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

//! Loads run as tasks of their own on a multi-threaded tokio runtime. The
//! runtime needs threads, so these are compiled only off WebAssembly.
#![cfg(not(target_family = "wasm"))]

mod common;

use std::sync::Arc;

use common::*;
use fiftyone_caching::{LoadingCache, StartLoad};
use tokio::runtime::{Builder, Handle};

/// Starts each load on one of the runtime's blocking threads and runs it
/// there, so the load's future never moves between threads.
fn tokio_spawner(start: StartLoad) {
    tokio::task::spawn_blocking(move || Handle::current().block_on(start()));
}

#[test]
fn a_dropped_loading_caller_neither_stops_the_load_nor_starts_another() {
    let runtime = Builder::new_multi_thread()
        .worker_threads(4)
        .build()
        .unwrap();
    runtime.block_on(async {
        let clock = TestClock::new();
        let source = Source::gated();
        let cache = Arc::new(
            LoadingCache::builder(lru(&clock, 100), source.clone())
                .clock(clock.shared())
                .spawner(tokio_spawner)
                .build(),
        );

        let first = tokio::spawn({
            let cache = Arc::clone(&cache);
            async move { cache.get(&1).await }
        });
        // The load cannot finish until the gate opens, so once it has started
        // the first caller is only waiting for it.
        while source.loads() == 0 {
            tokio::task::yield_now().await;
        }
        let others: Vec<_> = (0..8)
            .map(|_| {
                let cache = Arc::clone(&cache);
                tokio::spawn(async move { cache.get(&1).await })
            })
            .collect();
        first.abort();
        assert!(first.await.unwrap_err().is_cancelled());
        source.gate.open();

        for other in others {
            assert_eq!(other.await.unwrap(), Ok("1 from load 1".to_owned()));
        }
        assert_eq!(source.loads(), 1);
    });
}
